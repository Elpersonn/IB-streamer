document.getElementById("loginbtn").addEventListener("click", async (ev) => {
    ev.preventDefault()
    if (document.getElementById("password").value != document.getElementById("confpassword").value) {
        alert("Passwords are not matching!")
        return
    }
    let data = new URLSearchParams()
    data.append("username", document.getElementById("username").value)
    data.append("password", document.getElementById("password").value)
    let sentData = await fetch("/register", {
        headers: {"Content-Type": "application/x-www-form-urlencoded"},
        method: "POST",
        body: data.toString()
    })
    if (sentData.status == 401) {
        alert("Incorrect login or password")
    } else if (sentData.status == 200) {
        location.href = "/"
    } else alert("Unknown error: " + sentData.status + " " + sentData.body)
})