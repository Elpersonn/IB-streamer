document.getElementById("loginbtn").addEventListener("click", async (ev) => {
    ev.preventDefault()
    let data = new URLSearchParams()
    data.append("username", "admin")
    data.append("password", document.getElementById("password").value)
    let sentData = await fetch("/admin/login", {
        method: "POST",
        headers: {
            "Content-Type": "application/x-www-form-urlencoded"
        },
        body: data.toString()
    })
    if (sentData.status == 401) {
        alert("Incorrect login or password")
    } else if (sentData.status == 200) {
        location.href = "/admin"
    } else alert("Unknown error: " + sentData.status + " " + sentData.body)
})