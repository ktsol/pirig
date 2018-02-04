# Healty Rig service

### Configure as systemd service

Create file /etc/systemd/system/healthyrig.service
```
[Unit]
Description=HealtyRig

[Service]
Type=simple
ExecStart=/path/to/bin/healthyrig -d /path/to/your/configuration/healthyrig.toml
User=root
Group=root
Restart=always

[Install]
WantedBy=multi-user.target
```
