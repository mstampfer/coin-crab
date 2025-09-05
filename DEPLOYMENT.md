# Deployment Guide

## GitHub Actions Setup

### Required Secrets

To enable automatic deployment, add the following secrets to your GitHub repository:

1. **Go to GitHub Repository Settings**
   - Navigate to: `Settings` → `Secrets and variables` → `Actions`

2. **Add Repository Secret**
   - Name: `AWS_SSH_PRIVATE_KEY`
   - Value: Contents of your `~/.ssh/aws-freetier.pem` file
   - ⚠️ **Important**: Copy the entire private key including the header and footer lines:
     ```
     -----BEGIN RSA PRIVATE KEY-----
     [key content]
     -----END RSA PRIVATE KEY-----
     ```

### Environment Configuration

#### Production (AWS EC2: 100.26.107.175)
- **Client**: Already configured to use AWS EC2 IP in `crates/ios_lib/.env.client`
- **Server**: You'll need to create `crates/server/.env.server` on the EC2 instance with:
  ```env
  CMC_API_KEY=your_coinmarketcap_api_key_here
  MQTT_BROKER_HOST=0.0.0.0
  LOG_LEVEL=ERROR  # Options: OFF, ERROR, WARN, INFO, DEBUG, TRACE
  ```

#### Local Development
- Use `.env.local` configuration for local testing:
  ```bash
  cp .env.local crates/ios_lib/.env.client
  ```

### Deployment Process

#### Automatic Deployment
- **Trigger**: Push to `main` branch with changes to server code
- **Process**: 
  1. Runs all Rust tests
  2. Builds server binary
  3. Deploys to AWS EC2
  4. Restarts server with zero downtime
  5. Verifies MQTT broker is running

#### Manual Server Management
```bash
# SSH into production server
ssh -i ~/.ssh/<private key file> ec2-user@<EC2 IP Address>

# Server commands
cd coin_crab_server
./coin-crab-server          # Start manually
pkill coin-crab-server      # Stop server  
tail -f server.log          # View logs
cat server.pid              # Get process ID
```

### Monitoring

#### Health Checks
The deployment automatically verifies:
- ✅ Server process is running
- ✅ MQTT broker listening on port 1883
- ✅ No critical errors in logs

#### Manual Monitoring
```bash
# Check server status
ps -p $(cat server.pid)

# Monitor MQTT traffic
netstat -tuln | grep :1883

# View recent logs  
tail -20 server.log

# Live log monitoring
tail -f server.log
```

### Troubleshooting

#### Common Issues

1. **Deployment fails with SSH connection error**
   - Verify `AWS_SSH_PRIVATE_KEY` secret is correctly set
   - Check EC2 instance is running and accessible

2. **Server won't start on EC2**
   - Check `crates/server/.env.server` exists with valid CMC API key
   - Verify port 1883 isn't blocked by firewall
   - Check server.log for error details

3. **iOS app can't connect**
   - Verify EC2 security group allows inbound connections on port 1883
   - Check MQTT_BROKER_HOST in client configuration
   - Ensure server is running with `ps -p $(cat server.pid)`

### Security Considerations

- ✅ Private SSH key stored as GitHub secret (encrypted)
- ✅ API keys only on server, never in client code
- ✅ MQTT broker configured for secure production use
- ✅ Server logs don't expose sensitive information

### Next Steps

1. Add the SSH private key to GitHub secrets
2. Push changes to trigger first deployment
3. Verify server is accessible from iOS app
4. Monitor logs and performance