#!/usr/bin/env bash

set -e -u -o pipefail

this_dir="$(realpath "$(dirname "$0")")"

aws lambda invoke \
  --function-name "arn:aws:lambda:eu-west-2:918538493915:function:$1Challenger" \
  --invocation-type 'RequestResponse' \
  --payload "file://$this_dir/payload.json" \
  --cli-binary-format raw-in-base64-out \
  --region 'eu-west-2' --no-cli-pager /dev/stdout