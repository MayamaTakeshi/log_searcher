#!/bin/sh
set -e

BINARY=./target/x86_64-unknown-linux-musl/release/log_searcher

if [ ! -f "$BINARY" ]; then
  echo "Binary not found: $BINARY"
  echo "Run build.sh first."
  exit 1
fi

cp "$BINARY" /usr/local/bin/log_searcher
chmod +x /usr/local/bin/log_searcher

if command -v systemctl > /dev/null 2>&1 && [ -d /etc/systemd/system ]; then
  echo "Installing systemd service..."
  cp log_searcher.service /etc/systemd/system/log_searcher.service
  systemctl daemon-reload
  systemctl enable log_searcher
  systemctl start log_searcher
  echo "Started via systemd. Status: $(systemctl is-active log_searcher)"
else
  echo "Installing SysV init script..."
  cp log_searcher.initd /etc/init.d/log_searcher
  chmod +x /etc/init.d/log_searcher
  if command -v update-rc.d > /dev/null 2>&1; then
    update-rc.d log_searcher defaults
  elif command -v chkconfig > /dev/null 2>&1; then
    chkconfig --add log_searcher
  fi
  /etc/init.d/log_searcher start
  echo "Started via SysV init."
fi
