import { addShape, LatLng } from './map_utils';
import * as wasm from 'fastgtfs-js';

export interface Solution {
    components: SolutionComponent[];
}
export interface SolutionComponent {
    Bus: BusComponent;
}
interface BusComponent {
    shape: LatLng[];
}

export function navigate(from: LatLng, to: LatLng): Solution[] {
    console.log('Navigating from ', from, 'to', to);
    const solutions: Solution[] = wasm.get_solutions(from.lat, from.lng, to.lat, to.lng);
    if (solutions.length === 0) {
        console.error('No solution found');
        return;
    }
    return solutions;
}

// Returns the paths to clean them up afterwards.
export function addSolutionToMap(solution: Solution, map: H.Map): H.map.Polyline[] {
    let paths: H.map.Polyline[] = [];

    solution.components.forEach((component: any) => {
        if ('Walk' in component) {
            // todo
        } else if ('Bus' in component) {
            paths.push(addShape(component.Bus.shape, map));
        }
    });
    return paths;
}
