#!/bin/bash

set -e

createuser -U postgres -s clh
psql -U postgres -c "alter role clh password '${CLH_POSTGRES_PASSWORD}';"
