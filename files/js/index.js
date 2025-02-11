const VIDEOSRC = "files/hls/playlist.m3u8"
const TEMPLATE = document.getElementById("chatmessage")

let socket = undefined

/* class ChatMessage extends HTMLElement {
    constructor(author, message) {
        super()
        let template = document.getElementById("chatmessage")
        let templateContent = template.content 
        templateContent.children[0].innerHTML = author
        templateContent.innerHTML = message
    }
} */

function addMessage(author, content) {
    let newtempl = TEMPLATE.content.cloneNode(true)
    newtempl.querySelector("span").innerHTML = author + ":"
    newtempl.querySelector("p").innerHTML += (" " + content)
    //newtempl.id = self.crypto.randomUUID()
    newtempl.id = "asdasd"
    document.getElementById("inputform").before(newtempl)
}

function logout() {
    fetch("/login", {
        method: "DELETE"
    }).then((resp) => {
        if (resp.status != 200) {
            alert("Error while performing log out")
            return
        }
    })
}
function onEnter(ev) {
    ev.preventDefault()
    if (socket != undefined) {
        socket.send(document.getElementById("chatinput").value)
        document.getElementById("chatinput").value = ""
    }
}



var stream = document.getElementById("stream")
document.getElementById("inputform").addEventListener("submit", onEnter)
//window.customElements.define("chat-message", ChatMessage)
if (Hls.isSupported()) {
    var hls = new Hls({liveDurationInfinity: true,  liveMaxLatencyDurationCount: 10, liveSyncDurationCount: 3, maxBufferLength: 15})
    hls.loadSource(VIDEOSRC)
    hls.attachMedia(stream)
    hls.on(Hls.Events.FRAG_LOADED, function (d) {
        document.getElementById("chatinput").removeAttribute("disabled")
        if (socket == undefined) {
            socket = new WebSocket("/chat")
            socket.onmessage = (ev) => {
                let split = ev.data.toString().split(":")
                addMessage(split[0], split[1])
            }
            socket.onopen = (ev) => {
                addMessage("SYSTEM", "Welcome to the live chat! Remember to follow the rules!")
            }
            socket.onclose = (ev) => {
                console.warn(ev)
            }
            socket.onerror = (ev) => {
                console.warn(ev)
            }
        }
    })
} else alert("HLS not supported!")

let elem = document.getElementById("chatinput")
if (!elem.hasAttribute("disabled")) {
    
}
