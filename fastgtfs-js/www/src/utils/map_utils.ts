import H from '@here/maps-api-for-javascript';

export function addShape(shape: LatLng[], map: H.Map, color: string = getRandomColor()): H.map.Polyline {
    const lineString = new H.geo.LineString();

    shape.forEach((p) => lineString.pushPoint(toHPoint(p)));

    const polyline = new H.map.Polyline(lineString, {
        data: undefined,
        style: {
            lineWidth: 4,
            fillColor: color,
        },
    });
    console.log('Adding polyline to map: ', polyline);
    map.addObject(polyline);
    return polyline;
}

export function getRandomColor(): string {
    const letters = '0123456789ABCDEF';
    let color = '#';
    for (let i = 0; i < 6; i++) {
        color += letters[Math.floor(Math.random() * 16)];
    }
    return color;
}

export function getRandomSVGColor(): string {
    const color: number[] = [];
    for (let i = 0; i < 3; i++) {
        color.push(Math.floor(Math.random() * 255));
    }
    const c = `rgb(${color.join(',')})`;
    console.log('color: ', c);
    return c;
}

export function ease(from: LatLng, to: LatLng, duration: number, onStep: (pos: LatLng) => void) {
    const initialTime = new Date().getTime();
    const finalTime = initialTime + duration;

    const initialLat = from.lat;
    const initialLng = from.lng;

    const deltaLat = to.lat - from.lat;
    const deltaLng = to.lng - from.lng;

    function changePosition() {
        const currentTime = new Date().getTime();
        if (currentTime > finalTime) return;
        const percentage = (currentTime - initialTime) / duration;
        const currentPosition: LatLng = new LatLng(
            initialLat + deltaLat * percentage,
            initialLng + deltaLng * percentage
        );
        onStep(currentPosition);

        requestAnimationFrame(() => {
            changePosition();
        });
    }

    changePosition();
}

export class LatLng {
    constructor(readonly lat: number, readonly lng: number) {}
}

export function toHPoint(pos: LatLng): H.geo.Point {
    return new H.geo.Point(pos.lat, pos.lng);
}
