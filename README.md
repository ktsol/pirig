# RaspberryPi controlled mining rigs

## HealtyRig miner status service

Create file /etc/systemd/system/healthyrig.service
```
[Unit]
Description=healtyrig

[Service]
Type=simple
ExecStart=/path/to/bin/healthyrig -g 4 -p 4242
User=root
Group=root
Restart=always

[Install]
WantedBy=multi-user.target
```

## ThorinPi controller