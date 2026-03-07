const isProd = process.env.NODE_ENV === 'production';

/** @type {import('next').NextConfig} */
const nextConfig = {
  ...(isProd ? { output: 'export' } : {}),
  trailingSlash: true,
  images: {
    unoptimized: true
  },
  eslint: {
    ignoreDuringBuilds: true
  },
  typescript: {
    ignoreBuildErrors: true
  },
  // GitHub repo is handled server-side via Cloudflare Pages Function (functions/api/release.ts)
  // Set GITHUB_REPO in Cloudflare Pages dashboard environment variables
};

export default nextConfig;
