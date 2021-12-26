export function addShape(
  shape: google.maps.LatLng[],
  map: google.maps.Map,
  color: string = getRandomColor()
) {
  const path = new google.maps.Polyline({
    path: shape,
    geodesic: true,
    strokeColor: color,
    strokeOpacity: 0.5,
    strokeWeight: 7,
  });
  path.setMap(map);
  return path;
}

export function getRandomColor(): string {
  const letters = '0123456789ABCDEF';
  let color = '#';
  for (let i = 0; i < 6; i++) {
    color += letters[Math.floor(Math.random() * 16)];
  }
  return color;
}

export function moveMarkerSmooth(
  from: google.maps.LatLng,
  to: google.maps.LatLng,
  marker: google.maps.Marker,
  timeMs: number
) {
  const initialTime = new Date().getTime();
  const finalTime = initialTime + timeMs;

  const initialLat = from.lat();
  const initialLng = from.lng();

  const deltaLat = to.lat() - from.lat();
  const deltaLng = to.lng() - from.lng();

  function changePosition() {
    const currentTime = new Date().getTime();
    if (currentTime > finalTime) return;
    const percentage = (currentTime - initialTime) / timeMs;
    const currentPosition = new google.maps.LatLng(
      initialLat + deltaLat * percentage,
      initialLng + deltaLng * percentage
    );
    marker.setPosition(currentPosition);
    requestAnimationFrame(() => {
      changePosition();
    });
  }

  changePosition();
}
