import React, { useEffect, useState } from 'react';
import * as wasm from 'fastgtfs-js';
import { AppBar, Box, Tab, Tabs } from '@mui/material';
import Simulation from './simulation';
import Navigator from './navigator';

export default function MainScreen() {
    const [dataLoaded, setDataLoaded] = useState(false);
    const [value, setValue] = useState(1);

    useEffect(() => {
        async function downloadDataAndParse() {
            if (dataLoaded) return;
            console.log('Downloaded data and parsing');
            const file_url = `${window.location.href}/gtfs_serialized.zip`;
            await wasm.download_and_parse(file_url);
            setDataLoaded(true);
        }

        downloadDataAndParse().then((r) => 'Data loaded successfully!');
    });

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {
        setValue(newValue);
    };

    return (
        <Box
            sx={{
                bgcolor: 'background.paper',
                display: 'flex',
                flexDirection: 'column',
                flexGrow: 1,
                width: '100%',
                height: '100%',
            }}
        >
            <AppBar position="static" sx={{ zIndex: (theme) => theme.zIndex.drawer + 1 }}>
                <Tabs
                    value={value}
                    onChange={handleChange}
                    indicatorColor="secondary"
                    textColor="inherit"
                    variant="fullWidth"
                    aria-label="full width tabs example"
                >
                    <Tab label="Simulation" />
                    <Tab label="Navigator" />
                </Tabs>
            </AppBar>
            {!dataLoaded ? 'Loading Wasm data' : value == 0 ? <Simulation /> : <Navigator />}
        </Box>
    );
}
