
var mymap = L.map("map").on("load", () => {
    update_bounds_inputs();
    fetch_messages();
});

mymap.setView([51.505, -0.09], 13);


L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png ', {
    attribution: "Map data &copy; OpenStreetMap.org",
    maxZoom: 18,
    //tileSize: 512,
    //zoomOffset: -1,
}).addTo(mymap);

function update_bounds_inputs() {
    var bounds = mymap.getBounds();

    let nw_lat = bounds._northEast.lat;
    let nw_lon = bounds._southWest.lng;
    let se_lat = bounds._southWest.lat;
    let se_lon = bounds._northEast.lng;

    document.getElementById("se_lat").value = se_lat;
    document.getElementById("se_lon").value = se_lon;
    document.getElementById("nw_lat").value = nw_lat;
    document.getElementById("nw_lon").value = nw_lon;

    fetch_messages();
}

// Each change in bound is reflected in the form
mymap.on('moveend', update_bounds_inputs);

function fetch_messages() {
    se_lat = document.getElementById("se_lat").value;
    se_lon = document.getElementById("se_lon").value;
    nw_lat = document.getElementById("nw_lat").value;
    nw_lon = document.getElementById("nw_lon").value;

    fetch(`/messages?nw_lat=${nw_lat}&nw_lon=${nw_lon}&se_lat=${se_lat}&se_lon=${se_lon}`).then((response) => {
        if (response.ok) {
            response.text().then((t) => {
                document.getElementById("messages").innerHTML = t;
                let messages = document.querySelectorAll(".message");
                if (messages.length !== 0) {
                let lastOne = messages[messages.length-1];
                    lastOne.scrollIntoView();
                }
            });
        }
    });
}

document.body.addEventListener("submit", async function (event) {
    event.preventDefault();

    const form = event.target;

    if (form["user"].value && form["message"].value) {
        const result = await fetch(form.action, {
            method: "POST",
            body: new URLSearchParams([...(new FormData(form))])
        }).then(() => {
            let message = document.querySelector("input[name='message']");
            message.value = "";
            message.focus();
        })
              .then(fetch_messages)
              .catch((error) => console.error(error));
    }
});


var intervalId = setInterval(function() {
    fetch_messages();
}, 1000);
