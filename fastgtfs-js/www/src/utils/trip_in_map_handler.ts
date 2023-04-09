import H from '@here/maps-api-for-javascript';
import * as wasm from 'fastgtfs-js';
import { faBus } from '@fortawesome/free-solid-svg-icons';
import { addShape, ease, getRandomColor, getRandomSVGColor, LatLng, toHPoint } from './map_utils';
import { getSecondsSinceMidnight, UPDATE_PERIOD } from './time_utils';


export class TripHandler {
    private pin: H.map.DomMarker;

    private readonly initialTime: number; // start time of the trip (seconds since midnight)

    private polyline: H.map.Polyline;

    private lastLocation: LatLng;

    constructor(private trip_id: number, private map: H.Map) {
        this.initialTime = Number(wasm.init_trip_position_in_real_time(trip_id));

        const shape = wasm.get_shape(trip_id);
        this.lastLocation = this.getCurrentLocation();
        console.log('Current location: ', this.lastLocation);

        const color = getRandomColor();

        this.pin = new H.map.Marker(this.lastLocation, {
            icon: this.createIcon(),
            data: undefined
        });

        map.addObject(this.pin);
        this.polyline = addShape(shape, this.map);
      
    }

    createSvgWithBusNumber(busNumber: string) {
        const bus = faBus.icon;
    }

    createIcon() {
        const bus = faBus.icon;
        const svg = `<svg width="24" height="24" xmlns="http://www.w3.org/2000/svg" class="svg-icon" viewBox="0 0 24 24"><g stroke="#000"><path d="M22.875 6H22.5V3.75C22.5 1.65 17.85 0 12 0S1.5 1.65 1.5 3.75V6h-.375A1.125 1.125 0 0 0 0 7.125v3.75A1.125 1.125 0 0 0 1.125 12H1.5v7.5A1.5 1.5 0 0 0 3 21v1.5A1.5 1.5 0 0 0 4.5 24H6a1.5 1.5 0 0 0 1.5-1.5V21h9v1.5A1.5 1.5 0 0 0 18 24h1.5a1.5 1.5 0 0 0 1.5-1.5V21h.3c.75 0 1.2-.6 1.2-1.2V12h.375A1.125 1.125 0 0 0 24 10.875v-3.75A1.125 1.125 0 0 0 22.875 6zM5.25 18.75a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3zM6 13.5A1.5 1.5 0 0 1 4.5 12V6A1.5 1.5 0 0 1 6 4.5h12A1.5 1.5 0 0 1 19.5 6v6a1.5 1.5 0 0 1-1.5 1.5H6zm12.75 5.25a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3z" stroke-width=".047" fill="FILLL"/><text xml:space="preserve" font-family="Noto Sans JP" font-size="8" y="11.656" x="4.143" stroke-width="0" fill="FILLL">TEXTT</text></g></svg>`;
        const finalSvg = svg.replace('FILLL', getRandomSVGColor()).replace('TEXTT', wasm.get_trip_name(this.trip_id));
        const busIcon = new H.map.Icon(
            finalSvg
        );
        return busIcon;
    }

    removeFromMap() {
        this.map.removeObject(this.pin);
        this.map.removeObject(this.polyline);
        this.polyline = null;
        this.pin = null;
    }
    
    getCurrentLocation(currentSecondsSinceMidnight: number = getSecondsSinceMidnight()): LatLng {
        const newPosition = wasm.get_trip_position(this.trip_id, currentSecondsSinceMidnight);
        return new LatLng(newPosition.lat, newPosition.lng);
    }

    updateMarkerLocation(currentSecondsSinceMidnight: number) {
        const newLocation = this.getCurrentLocation(currentSecondsSinceMidnight);
        ease(this.lastLocation, newLocation, UPDATE_PERIOD, (pos: LatLng) => {
            this.pin.setGeometry(toHPoint(pos));
        });
        this.lastLocation = newLocation;
    }
}
