document.getElementById("resetbtn").addEventListener("click", (ev) => {
    fetch("/admin/keychange", {
        method: "POST"
    }).then((res) => {
        if (res.status == 200) {
            res.text().then((data) => document.getElementById("streamkey").value = data)
        }
    })
})
document.getElementById("titlebtn").addEventListener("click", (ev) => {
    let params = new URLSearchParams({username: document.getElementById("title").value, password: document.getElementById("description").value})
    fetch("/admin/info", {
        method: "POST",
        headers: {
            "Content-Type": "application/x-www-form-urlencoded"
        },
        body: params.toString()
    }).then((res) => {
        if (res.status != 200) {
            alert("Failed to change title and description")
        }
    })
})