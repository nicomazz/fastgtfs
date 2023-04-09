import * as wasm from 'fastgtfs-js';
import { getDateYYYYMMDD, getSecondsSinceMidnight } from './time_utils';
import { TripHandler } from './trip_in_map_handler';
import { LatLng } from './map_utils';

export function getTripsForSimulation(position: LatLng = new LatLng(45.46394, 12.22458), number = 50): [number] {
    console.log('Initializing network simulation:');
    const secondsSinceMidnight = getSecondsSinceMidnight();
    const date = getDateYYYYMMDD();
    console.log('Seconds:', secondsSinceMidnight, 'date:', date);
    const trips: [number] = wasm.get_near_trips(position.lat, position.lng, secondsSinceMidnight, date, number);
    console.log('Trips found: ', trips);
    return trips;
}

export function updateTripsPosition(handlers: TripHandler[]) {
    const secondsSinceMidnight = getSecondsSinceMidnight();
    handlers.forEach((th) => th.updateMarkerLocation(secondsSinceMidnight));
}
