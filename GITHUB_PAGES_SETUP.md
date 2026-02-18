# GitHub Pages Setup Guide

This guide will help you enable GitHub Pages for your Goldfish documentation.

## Quick Setup (2 minutes)

### 1. Go to Repository Settings

1. Navigate to your GitHub repository: `https://github.com/harshapalnati/goldfish`
2. Click on **Settings** tab
3. In the left sidebar, click **Pages**

### 2. Configure GitHub Pages

Under **Build and deployment**:

- **Source**: Select "GitHub Actions"

That's it! The workflow file (`.github/workflows/docs.yml`) is already configured.

### 3. Verify Deployment

1. Go to **Actions** tab in your repository
2. You should see the "Deploy Documentation to GitHub Pages" workflow
3. It will run automatically on your next push to `main`

### 4. Access Your Docs

Once deployed, your documentation will be available at:

```
https://harshapalnati.github.io/goldfish/
```

## Manual Trigger

To deploy immediately without waiting for a push:

1. Go to **Actions** tab
2. Click "Deploy Documentation to GitHub Pages"
3. Click "Run workflow" → "Run workflow"

## What Gets Deployed?

The workflow automatically:
- Builds documentation with `cargo doc`
- Includes all public items
- Creates an index redirect to the main crate docs
- Deploys to GitHub Pages on every push to main

## Troubleshooting

### Workflow not running?
- Make sure the file is committed: `.github/workflows/docs.yml`
- Check that GitHub Actions are enabled in Settings → Actions → General

### 404 error on the site?
- Wait 2-3 minutes after deployment
- Clear browser cache
- Check the Actions log for errors

### Custom domain?
If you want to use a custom domain (e.g., `docs.goldfish.rs`):

1. Add a `CNAME` file to the repository root with your domain
2. Configure DNS to point to GitHub Pages
3. Enable HTTPS in the Pages settings

## Local Testing

To test the documentation build locally:

```bash
cargo doc --no-deps --document-private-items

# Then open
cargo doc --open
```

## Badges

Your README already includes badges that will become active:

- [![Docs.rs](https://docs.rs/goldfish/badge.svg)](https://docs.rs/goldfish) - Automatic from crates.io
- [![Build Status](https://github.com/harshapalnati/goldfish/workflows/CI/badge.svg)](https://github.com/harshapalnati/goldfish/actions) - From the CI workflow

## Next Steps

After enabling GitHub Pages:

1. ✅ Update the README link to point to your GitHub Pages URL
2. ✅ Add the docs URL to your repository "About" section
3. ✅ Tweet/announce your documentation is live!
4. 🎉 Profit!

---

**Questions?** Open an issue on GitHub!
