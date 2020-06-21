#!/bin/sh

wget http://actv.avmspa.it/sites/default/files/attachments/opendata/automobilistico/actv_aut.zip
wget http://actv.avmspa.it/sites/default/files/attachments/opendata/navigazione/actv_nav.zip

unzip -o -d actv_aut actv_aut.zip 
unzip -o -d actv_nav actv_nav.zip

rm actv_aut.zip
rm actv_nav.zip