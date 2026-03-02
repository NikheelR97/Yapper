/**
 * r2-setup.mjs — One-time R2 bucket configuration for Yapper media.
 *
 * Sets:
 *   1. CORS rules  — allow PUT from app origins; GET from anywhere
 *   2. Lifecycle   — delete media/ objects after 30 days
 *
 * Prerequisites:
 *   npm install @aws-sdk/client-s3 dotenv   (run once in this directory)
 *
 * Usage:
 *   # Set env vars (or export them from backend/.env)
 *   R2_ACCOUNT_ID=...  R2_ACCESS_KEY_ID=...  R2_SECRET_ACCESS_KEY=...  R2_BUCKET_NAME=yapper-media \
 *     node scripts/r2-setup.mjs
 *
 *   # Or source backend/.env first:
 *   set -a && source backend/.env && set +a && node scripts/r2-setup.mjs
 */

import {
  S3Client,
  PutBucketCorsCommand,
  PutBucketLifecycleConfigurationCommand,
  GetBucketCorsCommand,
  GetBucketLifecycleConfigurationCommand,
} from '@aws-sdk/client-s3';

// ── Read config ──────────────────────────────────────────────────────────────

const accountId  = process.env.R2_ACCOUNT_ID;
const accessKey  = process.env.R2_ACCESS_KEY_ID;
const secretKey  = process.env.R2_SECRET_ACCESS_KEY;
const bucketName = process.env.R2_BUCKET_NAME ?? 'yapper-media';

if (!accountId || !accessKey || !secretKey) {
  console.error('Missing required env vars: R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY');
  process.exit(1);
}

const client = new S3Client({
  region: 'auto',
  endpoint: `https://${accountId}.r2.cloudflarestorage.com`,
  credentials: { accessKeyId: accessKey, secretAccessKey: secretKey },
});

// ── 1. CORS rules ────────────────────────────────────────────────────────────

const corsRules = [
  {
    // Presigned PUT uploads — only from known app origins
    ID: 'yapper-media-put',
    AllowedOrigins: [
      'https://app.yapperhq.com',
      'http://localhost:5173',
      'http://localhost:1420', // Tauri dev
    ],
    AllowedMethods: ['PUT'],
    AllowedHeaders: ['Content-Type', 'Content-Length'],
    MaxAgeSeconds: 3600,
  },
  {
    // Public GET reads — encrypted blobs, safe to serve to any origin
    ID: 'yapper-media-get',
    AllowedOrigins: ['*'],
    AllowedMethods: ['GET', 'HEAD'],
    AllowedHeaders: ['*'],
    MaxAgeSeconds: 86400,
  },
];

console.log(`\nConfiguring CORS on bucket: ${bucketName}`);
await client.send(new PutBucketCorsCommand({
  Bucket: bucketName,
  CORSConfiguration: { CORSRules: corsRules },
}));
console.log('  ✓ CORS rules applied');

// Verify
const { CORSRules } = await client.send(new GetBucketCorsCommand({ Bucket: bucketName }));
console.log(`  ✓ Verified — ${CORSRules?.length ?? 0} rule(s) active`);

// ── 2. Lifecycle rule ────────────────────────────────────────────────────────

console.log('\nConfiguring lifecycle rules...');
await client.send(new PutBucketLifecycleConfigurationCommand({
  Bucket: bucketName,
  LifecycleConfiguration: {
    Rules: [
      {
        ID: 'expire-media-30-days',
        Status: 'Enabled',
        Filter: { Prefix: 'media/' },
        Expiration: { Days: 30 },
      },
    ],
  },
}));
console.log('  ✓ Lifecycle rule applied (media/ objects deleted after 30 days)');

// Verify
const { Rules } = await client.send(new GetBucketLifecycleConfigurationCommand({ Bucket: bucketName }));
console.log(`  ✓ Verified — ${Rules?.length ?? 0} rule(s) active`);

console.log('\nDone! R2 bucket is ready for Yapper media.\n');
