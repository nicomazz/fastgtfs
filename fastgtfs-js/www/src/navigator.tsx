import {
    Autocomplete,
    Box,
    CardContent,
    Chip,
    CssBaseline,
    TextField,
    Toolbar,
    Typography,
    useTheme,
} from '@mui/material';
import React, { useEffect, useState } from 'react';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import Card from '@mui/material/Card';
import Map from './map';
import { LatLng } from './utils/map_utils';
import { Solution, navigate, SolutionComponent } from './utils/navigation_utils';

export default function Navigator() {
    const [fromToFields, setFromToFields] = useState(null);
    const theme = useTheme();
    const [solutions, setSolutions] = useState(null);

    const drawerWidth = 300;

    const onMapTap = (pos: LatLng) => {
        console.log('onMapTap:', pos);
        setFromToFields((old) => {
            if (old == null || old.from == null) {
                return { from: pos };
            } else if (old.to == null) {
                return {
                    to: pos,
                    ...old,
                };
            } else {
                return { from: pos };
            }
        });
    };

    useEffect(() => {
        if (fromToFields == null || fromToFields.from == null || fromToFields.to == null) {
            return;
        }
        console.log('Looking for solutions.', fromToFields);
        const newSolutions = navigate(fromToFields.from, fromToFields.to);
        console.log('setting solutions fould:', newSolutions);
        setSolutions(newSolutions);
    }, [fromToFields]);

    function format(pos?: LatLng) {
        if (pos == null) return '';
        return pos.lat.toFixed(3) + ', ' + pos.lng.toFixed(3);
    }

    return (
        <Box sx={{ display: 'flex', width: '100%', height: '100%' }}>
            <CssBaseline />

            <Drawer
                sx={{
                    width: drawerWidth,
                    flexShrink: 0,
                    '& .MuiDrawer-paper': {
                        width: drawerWidth,
                        boxSizing: 'border-box',
                    },
                }}
                variant="persistent"
                anchor="left"
                open={true}
            >
                <Toolbar />
                <Autocomplete
                    disablePortal
                    id="from"
                    freeSolo={true}
                    options={[]}
                    value={format(fromToFields?.from)}
                    sx={{ width: '100%', p: 1 }}
                    renderInput={(params) => <TextField {...params} label="From" />}
                />
                <Autocomplete
                    disablePortal
                    id="to"
                    options={[]}
                    freeSolo={true}
                    value={format(fromToFields?.to)}
                    sx={{ width: '100%', p: 1 }}
                    renderInput={(params) => <TextField {...params} label="To" />}
                />

                <Divider />
                {solutions?.map((s: Solution) => (
                    <SolutionCard solution={s} />
                ))}
            </Drawer>
            <Box
                component="main"
                sx={{
                    flexGrow: 1,
                    flexDirection: 'column',
                    width: '100%',
                    height: '100%',
                }}
            >
                <Map key="map" onTapCallback={onMapTap} navigationSolutions={solutions} />
            </Box>
        </Box>
    );
}

export function SolutionCard(props: { solution: Solution }) {
    const s = props.solution;
    return (
        <Card variant="outlined" style={{ margin: 5 }}>
            <CardContent>
                <Typography variant="body2">
                    {s.components?.map((c: SolutionComponent) => (
                        <div>
                            <SolutionComponentUi c={c} />
                        </div>
                    ))}
                </Typography>
            </CardContent>
        </Card>
    );
}

export function SolutionComponentUi(props: { c: SolutionComponent }) {
    const c = props.c;
    if ('Walk' in c) {
        return (
            <Divider style={{ marginTop: 5, marginBottom: 5 }}>
                <Chip label="Walk" size="small" />
            </Divider>
        );
    } else {
        return <p>{c['Bus']['route']['route_short_name'] + ' ' + c['Bus']['route']['route_long_name']}</p>;
    }
}
