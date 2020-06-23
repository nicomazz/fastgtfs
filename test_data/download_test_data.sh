#!/bin/sh -x

download_and_unzip(){
  URL=$1
  FILE_NAME=${URL##*/}
  BASE_FILE_NAME=${FILE_NAME%.zip}
  echo "url: $URL filename: $FILE_NAME, base: $BASE_FILE_NAME";

  wget $URL
  unzip -o -d $BASE_FILE_NAME $FILE_NAME

  rm $FILE_NAME

}



download_and_unzip http://actv.avmspa.it/sites/default/files/attachments/opendata/automobilistico/actv_aut.zip
download_and_unzip http://actv.avmspa.it/sites/default/files/attachments/opendata/navigazione/actv_nav.zip
download_and_unzip https://www.alilaguna.it/attuale/alilaguna.zip

