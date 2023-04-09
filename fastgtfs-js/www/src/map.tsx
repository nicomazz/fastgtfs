import H from '@here/maps-api-for-javascript';
import React, { MutableRefObject, useEffect, useRef } from 'react';
import { updateTripsPosition } from './utils/public_network_simulator';
import { TripHandler } from './utils/trip_in_map_handler';
import { UPDATE_PERIOD } from './utils/time_utils';
import { LatLng } from './utils/map_utils';
import { addSolutionToMap, Solution } from './utils/navigation_utils';

type Props = {
    tripIds?: number[];
    navigationSolutions?: Solution[];
    onTapCallback: (pos: LatLng) => void;
};

export default function Map({ tripIds, navigationSolutions, onTapCallback }: Props) {
    const mapContainer = useRef(null);
    const map: MutableRefObject<H.Map> = useRef(null);
    const shapesOnMap: MutableRefObject<H.map.Polyline[]> = useRef(null);
    const timeoutId = useRef(null);

    useEffect(() => {
        if (map.current) return;

        const platform = new H.service.Platform({
            // Restricted to my domains.
            apikey: 'lSmaxWFxzCua1173HK9LjXtKyu9XM0shmej95n2qSyE',
        });
        const layers = platform.createDefaultLayers();
        const hereMap = new H.Map(mapContainer.current, layers.vector.normal.map, {
            pixelRatio: window.devicePixelRatio,
            center: { lat: 45.440847, lng: 12.315515 },
            zoom: 12,
        });
        window.addEventListener('resize', () => hereMap.getViewPort().resize());
        hereMap.addEventListener('tap', onTap);

        const behavior = new H.mapevents.Behavior(new H.mapevents.MapEvents(hereMap));

        map.current = hereMap;
    });

    function onTap(evt: H.mapevents.Event) {
        // calculate infobubble position from the cursor screen coordinates
        // destructure svt current point x and y
        const { viewportX: x, viewportY: y } = evt.currentPointer;
        const position = map.current.screenToGeo(x,y);
        const latlng = new LatLng(position.lat, position.lng);
        onTapCallback(latlng);
    }


    useEffect(() => {
        if (tripIds == null || map.current == null) return;
        console.log('Adding trips');
        const tripHandlers = tripIds.map((t: number) => new TripHandler(t, map.current));
        timeoutId.current = scheduleUpdateTripsPosition(tripHandlers);
        return () => {
            console.log('Removing trips from map');
            clearTimeout(timeoutId.current);
            tripHandlers.forEach((th) => th.removeFromMap());
        };
    }, [tripIds]);

    useEffect(() => {
        if (navigationSolutions == null || navigationSolutions.length == 0) {
            return;
        }
        // Let's just show the first one for now (..and maybe forever?)
        const firstSolution = navigationSolutions[0];
        shapesOnMap.current = addSolutionToMap(firstSolution, map.current);
        return () => {
            map.current.removeObjects(shapesOnMap.current);
            shapesOnMap.current = null;
        };
    }, [navigationSolutions]);

   function scheduleUpdateTripsPosition(handlers: TripHandler[]) {
        return setTimeout(() => {
            console.log("Updating trips' position on map");
            updateTripsPosition(handlers);
            timeoutId.current = scheduleUpdateTripsPosition(handlers);
        }, UPDATE_PERIOD);
    }

    return <div style={{ width: '100%', height: '100%' }} ref={mapContainer} />;
}
