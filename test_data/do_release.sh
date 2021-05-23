#!/bin/sh -e

# We assume tests have already run
OUTPUT_FOLDER=gtfs_serialized
OUTPUT_GTFS_ZIP=${OUTPUT_FOLDER}.zip
PREVIOUS_RELEASE="old_dataset.zip"
export GITHUB_TOKEN=$1

zip_preprocessed_data() {
  zip -r $OUTPUT_GTFS_ZIP --junk-paths $OUTPUT_FOLDER
}

download_old_gtfs_data_release() {
  curl -s https://api.github.com/repos/nicomazz/Orari-Autobus-Actv/releases/latest |
    grep browser_download_url | grep gtfs | cut -d '"' -f 4 | wget -O old_dataset.zip -qi -

}

do_release() {
  THIS_TAG=$(date +%s%3N)
  ghr -u nicomazz -r Orari-Autobus-Actv $THIS_TAG $OUTPUT_GTFS_ZIP
}

clean_files() {
  rm $OUTPUT_GTFS_ZIP $PREVIOUS_RELEASE
}


main() {
  zip_preprocessed_data
  download_old_gtfs_data_release

  if cmp -s "$OUTPUT_GTFS_ZIP" "$PREVIOUS_RELEASE"; then
    printf 'This release is the same as the previous one. Skipping upload.'
    clean_files
    exit 0
  fi

  do_release
  clean_files
}

main
