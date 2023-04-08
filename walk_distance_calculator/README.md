# Walk distance calculator

Getting the time needed to move by walk from 2 points would normally require internat connection and api calls.

This lib precalculates walking times between each stop and the nearest X using HERE apis.

To make it work:

1. Copy `test_data` inside this folder (it's possible to download them with `../test_data/download_test_data.sh`). This will generate several folders, each with a GTFS dataset. The only important file is `stops.txt`
2. Run the script `cargo run`. It will take 1s for each stop.

If the script is stopped, it loads automatically the latest results to avoid duplicated computations (stored in `temp_walk_results` as flatbuffers).

Eventually, it generates `walk_results.txt`, with the following format :

```
<Number of points>;<number of near points calculated for each one>
<lat1,lng1>
...
<latN,lngN>
<inx> <inx nearby> <distance> ... <inx nearbyY> <inx nearbyY distance>
...
<inxN> ...
```

The requests are done with some smartness: if A->B is calculated, the result is also used for B->A.

Note: You need to export HERE APIs key to `HERE_APIKEY` env var before starting the script.
