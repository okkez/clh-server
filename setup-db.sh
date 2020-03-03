#!/bin/bash

set -e

diesel database setup
diesel migration run

