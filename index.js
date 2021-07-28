import * as wasm from "wasm-test";

let paths = [];

function navigate(from, to, map) {
    console.log("Navigating from ", from, "to", to);
    paths.forEach((path) => path.setMap(null));
    paths = [];
    let solutions = wasm.get_solutions(from.lat, from.lng, to.lat, to.lng);
    if (solutions.length === 0) {
        console.error("No solution found");
        return;
    }
    let first = solutions[0];
    console.log("First solution:", first);
    first.components.forEach((leg) => {
        if ("Walk" in leg) {
            // todo
        } else if ("Bus" in leg) {
            addShape(leg.Bus.shape, map);
        }
    });
}

function addShape(shape, map) {
    const path = new google.maps.Polyline({
        path: shape,
        geodesic: true,
        strokeColor: "#FF0000",
        strokeOpacity: 1.0,
        strokeWeight: 2,
    });
    path.setMap(map);
    paths.push(path);
}

window.navigate = navigate

async function main() {
    let location = window.location.href;
    let file_url = location + "/gtfs_serialized.zip";
    await wasm.download_and_parse(file_url);
    console.log("From js: ", wasm.try_navigate());
}

main();

