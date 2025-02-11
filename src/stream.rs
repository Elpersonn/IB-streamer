use gstreamer::{self, prelude::*, Pad};
use std::{env, sync::mpsc};

use crate::util::{self, MessageType};



pub fn initStream(rx: mpsc::Receiver<util::MessageType>) {
    gstreamer::init().expect("Failed to initialize Gstreamer");
    let mut playlist_path = env::current_dir().unwrap();
    playlist_path.push("files");
    playlist_path.push("hls");
    playlist_path.push("playlist.m3u8");

    let pipeline = gstreamer::Pipeline::default();

    let srt_src = gstreamer::ElementFactory::make("srtsrc").build().unwrap();
    let tsdemux = gstreamer::ElementFactory::make("tsdemux").build().unwrap();
    let h264parse = gstreamer::ElementFactory::make("h264parse").build().unwrap();
    let mpegts = gstreamer::ElementFactory::make("mpegtsmux").build().unwrap();
    let hls_sink = gstreamer::ElementFactory::make("hlssink").build().unwrap();

    srt_src.set_property("uri", "srt://127.0.0.1:7001?mode=listener"); 
    srt_src.set_property("keep-listening", true);
    srt_src.set_property("authentication", true);
    srt_src.set_property("streamid", "penis");
    hls_sink.set_property("playlist-location", playlist_path.to_str().unwrap());
    hls_sink.set_property("max-files", 7u32);
    
    playlist_path.pop();
    playlist_path.push("segment%05d.ts");

    hls_sink.set_property("location", playlist_path.to_str().unwrap());
    hls_sink.set_property("target-duration", 3u32);
    let h264pad = h264parse.static_pad("sink").unwrap();

    tsdemux.connect_pad_added(move |_, pad: &Pad| {
        if pad.name().starts_with("video") {
            pad.link(&h264pad).unwrap();
        }
    });
    /*srt_src.connect_closure("caller-connecting", false, gstreamer::glib::RustClosure::new(|values| {
        let streamid: String = values[2].get::<String>().unwrap();
        let element = values[0].get::<gstreamer::Element>().unwrap();
        println!("{}", streamid);
        if streamid == element.property::<String>("streamid") {
            println!("ALLOWED");
            return Some(gstreamer::glib::Value::from(true));
        } 
        Some(gstreamer::glib::Value::from(false))
    })); */

    pipeline.add_many([&srt_src, &tsdemux, &h264parse, &mpegts, &hls_sink]).unwrap();

    let mpeg_pad = mpegts.request_pad_simple("sink_%d").unwrap();
    h264parse.static_pad("src").unwrap().link(&mpeg_pad).unwrap();

    gstreamer::Element::link_many([&srt_src, &tsdemux]).unwrap();
    gstreamer::Element::link_many([&mpegts, &hls_sink]).unwrap();
    pipeline.set_state(gstreamer::State::Playing).unwrap();
    println!("GSTREAMER OK");
    loop {  
        match rx.recv().unwrap() {
            MessageType::STARTSTREAM => {
                pipeline.set_state(gstreamer::State::Playing).unwrap();
            },
            MessageType::STOPSTREAM => { pipeline.set_state(gstreamer::State::Null).unwrap(); },
            MessageType::KEYCHANGE(streamid) => {
                srt_src.set_state(gstreamer::State::Null);
                srt_src.set_property("streamid", streamid);
                srt_src.set_state(gstreamer::State::Playing);
            }
        }
    }
}