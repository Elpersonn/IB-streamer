var areachanged = false;
let textarea = document.getElementById("cssarea")
const filetoload = fetch("/files/css/index.css").then(res => res.text())
.then(data => textarea.value = data)

window.addEventListener("beforeunload", (ev) => {
    if (areachanged) {
        ev.preventDefault()
        return undefined
    }
})

document.getElementById("savebtn").addEventListener("click", (ev) => {
    ev.preventDefault()
    fetch("/admin/css", {
        method: "PUT",
        headers: {
            "Content-Type": "text/css"
        },
        body: textarea.value
    }).then((res) => {
        if (res.status != 200 ) {
            alert("Fail")
        } else location.reload()
    })
})
document.getElementById("resetbtn").addEventListener("click", (ev) => {
    ev.preventDefault()
    fetch("/admin/css", {
        method: "DELETE"
    }).then((res) => {
        if (res.status != 200 ) {
            alert("Fail")
        } else location.reload()
    })
})

textarea.addEventListener("input", (ev) => {
    areachanged = true
})
