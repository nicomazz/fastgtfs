import React, { useEffect, useState } from 'react';
import Map from './map';
import { getTripsForSimulation } from './utils/public_network_simulator';
import { LatLng } from './utils/map_utils';

export default function Simulation() {
    const [trips, setTrips] = useState(null);

    useEffect(() => {
        if (trips != null) return;
        const tripsForSimulation = getTripsForSimulation();
        setTrips(tripsForSimulation);
    }, [trips]);

    const onMapTap = (pos: LatLng) => {
        console.log('onMapTap:', pos);
        const tripsForSimulation = getTripsForSimulation(pos);
        setTrips(tripsForSimulation);
    };

    return <Map tripIds={trips} onTapCallback={onMapTap} key="map" />;
}
