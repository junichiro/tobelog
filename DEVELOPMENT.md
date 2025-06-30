# Development Guide for tobelog

## Quick Start

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd tobelog
   ```

2. **Set up environment**
   ```bash
   ./scripts/setup_dropbox.sh
   ```

3. **Configure Dropbox API**
   - Follow the instructions from the setup script
   - Update `.env` file with your Dropbox access token

4. **Test the setup**
   ```bash
   cargo run --bin test_dropbox
   ```

5. **Run the development server**
   ```bash
   cargo run
   ```

## Environment Variables

The following environment variables are required:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DROPBOX_ACCESS_TOKEN` | Dropbox API access token | None | ✅ Yes |
| `DATABASE_URL` | SQLite database URL | `sqlite://blog.db` | No |
| `SERVER_HOST` | Server bind address | `0.0.0.0` | No |
| `SERVER_PORT` | Server port | `3000` | No |
| `BLOG_TITLE` | Blog title | `My Personal Blog` | No |

## Dropbox Setup

### 1. Create a Dropbox App

1. Go to [Dropbox App Console](https://www.dropbox.com/developers/apps)
2. Click "Create app"
3. Choose "Scoped access"
4. Choose "App folder" (recommended for blog storage)
5. Name your app (e.g., "tobelog-blog-storage")
6. Click "Create app"

### 2. Configure Permissions

In your app settings, enable these permissions under the "Permissions" tab:
- `files.content.read`
- `files.content.write`
- `files.metadata.read`
- `files.metadata.write`

### 3. Generate Access Token

1. Go to the "Settings" tab in your app
2. Scroll down to "OAuth 2" section
3. Click "Generate access token"
4. Copy the token and add it to your `.env` file

### 4. Test Connection

```bash
cargo run --bin test_dropbox
```

This will:
- Test the API connection
- Display your account information
- Create the required folder structure
- Verify basic operations work

## API Endpoints

### Development Endpoints

- `GET /` - Home page
- `GET /health` - Health check
- `GET /api/dropbox/status` - Dropbox connection status

### Testing Dropbox API

```bash
# Test connection
curl http://localhost:3000/api/dropbox/status

# Expected successful response:
{
  "status": "connected",
  "account": {
    "name": "Your Name",
    "email": "your.email@example.com",
    "account_id": "..."
  },
  "message": "Dropbox API connection successful"
}
```

## Folder Structure

The application expects this folder structure in your Dropbox app folder:

```
/BlogStorage/
├── /posts/                    # Published blog posts
│   ├── /2024/
│   └── /2025/
├── /media/                    # Media files (images, videos)
│   ├── /images/
│   └── /videos/
├── /drafts/                   # Draft posts
├── /templates/                # Custom templates and styles
└── /config/                   # Configuration files
```

This structure will be automatically created when you run the test script.

## Development Commands

```bash
# Run the main application
cargo run

# Test Dropbox integration
cargo run --bin test_dropbox

# Run tests
cargo test

# Check code quality
cargo check
cargo clippy

# Format code
cargo fmt

# Run with specific environment
DROPBOX_ACCESS_TOKEN=your_token cargo run
```

## Troubleshooting

### Common Issues

1. **"DROPBOX_ACCESS_TOKEN environment variable not found"**
   - Make sure you have a `.env` file with the token
   - Run `./scripts/setup_dropbox.sh` to create it

2. **"Dropbox API connection failed"**
   - Verify your access token is correct
   - Check that your app has the required permissions
   - Ensure you have internet connectivity

3. **"Permission denied" errors with Dropbox**
   - Make sure your app has the required scopes enabled
   - Try regenerating your access token

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug cargo run
```

### Testing Without Real Dropbox

For unit tests, mock implementations are provided. Run:

```bash
cargo test
```

## Next Steps

Once you have the basic setup working:

1. Implement markdown processing (Issue #4)
2. Add web handlers and routing (Issue #5)
3. Build template engine integration (Issue #6)

## Need Help?

- Check the [Dropbox API Documentation](https://www.dropbox.com/developers/documentation/http/overview)
- Review the CLAUDE.md file for project architecture
- Open an issue if you encounter problems