# S3 setup notes

The [example access policy](iam-policy.json) grants read, write, delete, and multipart-upload permissions only under Mori's `private/` and `public/` object prefixes. Replace `YOUR_BUCKET_NAME` with the complete bucket name. Keep Block Public Access enabled.

The policy assumes S3-managed encryption. If the bucket uses a customer-managed KMS key, grant the corresponding KMS permissions as described in [AWS's multipart upload documentation](https://docs.aws.amazon.com/AmazonS3/latest/userguide/mpuoverview.html#multipart-upload-and-permissions).

The [CORS example](cors.json) permits browser uploads from one origin. Replace `https://media.example.com` with the exact origin configured as `MORI_PUBLIC_URL`; keep the scheme and port consistent. See [AWS's CORS documentation](https://docs.aws.amazon.com/AmazonS3/latest/userguide/ManageCorsUsing.html).

An S3 lifecycle rule can abort abandoned multipart uploads without expiring completed media. See [AWS's multipart upload documentation](https://docs.aws.amazon.com/AmazonS3/latest/userguide/mpuoverview.html).
