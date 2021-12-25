import * as wasm from 'fastgtfs-js';
import { addShape, getRandomColor, moveMarkerSmooth } from './map_utils';
import { faBus } from '@fortawesome/free-solid-svg-icons';

let TRIP_HANDLERS: TripHandler[] = [];
const UPDATE_PERIOD = 1000;

export function initNetworkSimulation(map: google.maps.Map) {
  console.log('Initializing network simulation:');
  let secondsSinceMidnight = getSecondsSinceMidnight();
  let date = getDateYYYYMMDD();
  console.log('Seconds:', secondsSinceMidnight, 'date:', date);
  let trips: [number] = wasm.get_near_trips(
    45.46394,
    12.22458,
    secondsSinceMidnight,
    date,
    500
  );
  console.log(trips);

  TRIP_HANDLERS = [];
  trips.forEach((t) => {
    TRIP_HANDLERS.push(new TripHandler(t, map));
  });
  console.log('Trip handlers initialized');
}

export function updateTripsPosition() {
  let secondsSinceMidnight = getSecondsSinceMidnight();
  TRIP_HANDLERS.forEach((th) => th.updateMarkerLocation(secondsSinceMidnight));
}

export function scheduleUpdateTripsPosition() {
  setTimeout(() => {
    updateTripsPosition();
    scheduleUpdateTripsPosition();
  }, UPDATE_PERIOD);
}

function getSecondsSinceMidnight(): number {
  let now = new Date();

  let then = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate(),
    0,
    0,
    0
  );

  return (now.getTime() - then.getTime()) / 1000;
}

function getDateYYYYMMDD() {
  let todayDate = new Date().toISOString().slice(0, 10);
  return todayDate.replaceAll('-', '');
}

class TripHandler {
  private readonly pin: google.maps.Marker;
  private readonly initialTime: number; // start time of the trip since midnight
  private polyline: google.maps.Polyline;
  private lastLocation: google.maps.LatLng;

  constructor(private trip_id: number, map: google.maps.Map) {
    this.initialTime = Number(wasm.init_trip_position_in_real_time(trip_id));

    let shape = wasm.get_shape(trip_id);
    this.lastLocation = new google.maps.LatLng(shape[0].lat, shape[0].lng);

    let color = getRandomColor();
    this.polyline = addShape(shape, map);
    this.pin = new google.maps.Marker({
      position: this.lastLocation,
      icon: {
        path: faBus.icon[4] as string,
        fillColor: color,
        fillOpacity: 1,
        labelOrigin: new google.maps.Point(
          faBus.icon[0] / 2, // width
          faBus.icon[1] / 2.5 // height
        ),
        anchor: new google.maps.Point(
          faBus.icon[0] / 2, // width
          faBus.icon[1] // height
        ),
        strokeWeight: 1,
        strokeColor: '#ffffff',
        scale: 0.075,
      },
      draggable: true,
      label: {
        fontSize: '15px',
        color: color,
        fontWeight: 'bold',
        text: wasm.get_trip_name(trip_id),
      },
      map,
      title: '',
    });
  }

  updateMarkerLocation(currentSecondsSinceMidnight: number) {
    let newPosition = wasm.get_trip_position(
      this.trip_id,
      currentSecondsSinceMidnight
    );
    let newLocation = new google.maps.LatLng(newPosition.lat, newPosition.lng);
    moveMarkerSmooth(this.lastLocation, newLocation, this.pin, UPDATE_PERIOD);
    this.lastLocation = newLocation;
    // this.pin.setPosition(new google.maps.LatLng(newPosition.lat, newPosition.lng));
  }
}

const _global = (window /* browser */ || global) /* node */ as any;
_global.updateTripsPosition = updateTripsPosition;
_global.initNetworkSimulation = initNetworkSimulation;
