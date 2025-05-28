#!/usr/bin/env bash

set -e -u -o pipefail

aws lambda invoke \
  --function-name "arn:aws:lambda:eu-west-2:918538493915:function:$1-Move" \
  --invocation-type 'RequestResponse' \
  --payload file://cloud/chessmove/payload.json \
  --cli-binary-format raw-in-base64-out \
  --region 'eu-west-2' --no-cli-pager /dev/stdout
