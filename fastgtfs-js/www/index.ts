import * as wasm from "fastgtfs-js";
import {addShape} from "./map_utils";
import {
    initNetworkSimulation,
    scheduleUpdateTripsPosition
} from "./public_network_simulator"
import Polyline = google.maps.Polyline;
import Marker = google.maps.Marker;

let paths: Polyline[] = [];

interface LatLng {
    lat: number,
    lng: number,
}

function navigate(from: LatLng, to: LatLng, map: google.maps.Map) {
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
    first.components.forEach((leg: any) => {
        if ("Walk" in leg) {
            // todo
        } else if ("Bus" in leg) {
            paths.push(addShape(leg.Bus.shape, map));
        }
    });
}


async function downloadDataAndParse() {
    let file_url = window.location.href + "/gtfs_serialized.zip";
    await wasm.download_and_parse(file_url);
    // console.log("From js: ", wasm.try_navigate());
}


let map: google.maps.Map;
let first_pos: LatLng = null;
let first_marker: Marker = null;
let second_pos: LatLng = null;
let second_marker: Marker = null;

async function initializeMap() {
    map = new google.maps.Map(document.getElementById("map"), {
        center: {lat: 45.440847, lng: 12.315515},
        zoom: 12,
    });

    map.addListener("click", (mapsMouseEvent: { latLng: { toJSON: () => LatLng; }; }) => {
        if (first_pos === null) {
            if (first_marker != null) first_marker.setMap(null);
            if (second_marker != null) second_marker.setMap(null);
            console.log("First set!");
            first_pos = mapsMouseEvent.latLng.toJSON();
            first_marker = new google.maps.Marker({
                position: first_pos,
                draggable: true,
                map,
                animation: google.maps.Animation.DROP,
                title: "Start",
            });
            setListeners(first_marker);

        } else if (second_pos === null) {
            console.log("Second set!");
            second_pos = mapsMouseEvent.latLng.toJSON();
            second_marker = new google.maps.Marker({
                position: second_pos,
                draggable: true,
                map,
                animation: google.maps.Animation.DROP,
                title: "End",
            });
            do_navigate();
            setListeners(second_marker);
            first_pos = null;
            second_pos = null;
        }
    });

    await downloadDataAndParse();
    initNetworkSimulation(map);
    scheduleMapUpdate();
}

function scheduleMapUpdate() {
    scheduleUpdateTripsPosition();
}

function setListeners(marker: Marker) {
    marker.addListener('dragend', function () {
        do_navigate();
    });
}

function do_navigate() {
    navigate(first_marker.getPosition().toJSON(), second_marker.getPosition().toJSON(), map);
}


const _global = (window /* browser */ || global /* node */) as any
_global.initializeMap = initializeMap;