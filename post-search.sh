#!/bin/bash

curl -X POST \
     --data-urlencode "hostname=localhost" \
     --data-urlencode "working_directory=$(pwd)" \
     --data-urlencode "command=echo this is test" \
     http://localhost:8088

curl http://localhost:8088
