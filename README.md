# Neuroheat

This is another version of the Heating Brain hobby project - an Elixir application for controlling the floor heating system in the house. It utilizes a Raspberry PI Zero, a relay controller, and a few 1-wire temperature sensors. The original Elixir application ran for over three years without any issues (except for CPU and memory utilization when rendering data). It provided around 20-30% savings in gas consumption compared to the previous "industry standard" devices (mainly due to optimized logic for enabling and disabling heating in the context of the required heat area).

**Disclaimer:** This is a hobby project, and I do not take any responsibility for any issues or damages that may arise from running this software. Use it at your own risk.

## Development

Install tools required by the `.tool-versions` file and ensure you have the following system packages installed on your macOS:

```
brew install arm-unknown-linux-musleabihf
rustup target add arm-unknown-linux-musleabihf
```

## Deployment

There is a `bin/deploy` script that builds the binary file and performs actions on the remote server (e.g., backing up the database, updating the systemd service, etc.). Make sure to review the heating configuration (e.g., GPIO pins, sensor identifiers, etc.).

If deploying for the first time, you will have to set up the server (see Raspberry Pi Zero Setup section) and create `heating_config.json` (see `heating_config.json.sample`).

### Accessing API endpoings

You can access all data by hitting the API endpoints with cURL.
The host name depends on your server configuration. Also, keep
in mind that the scheduler needs to populate some data before
you will see any results (prior to that, it will return HTTP 404).

```
neuroheat λ curl heating-brain.local:3030/api/temperatures/office | jq
{
  "timestamp": "2024-10-06 10:32:03",
  "temperature": "21.937",
  "key": "office",
  "label": "Office",
  "expected_temperature": "21"
}
```

```
neuroheat λ curl neurobrain.local:3030/api/temperatures | jq
{
  "office": {
    "temperature": "21.937",
    "label": "Office",
    "timestamp": "2024-10-06 10:30:03",
    "expected_temperature": "21"
  },
  "living_room": {
    "label": "Living Room",
    "temperature": "22",
    "timestamp": "2024-10-06 10:30:02",
    "expected_temperature": "21"
  },
  "bedroom": {
    "timestamp": "2024-10-06 10:30:04",
    "expected_temperature": "18.5",
    "temperature": "19.687",
    "label": "Bedroom"
  },
  "guest_room": {
    "temperature": "19.312",
    "timestamp": "2024-10-06 10:30:05",
    "expected_temperature": "18.5",
    "label": "Guest Room"
  },
  "bathroom": {
    "temperature": "21.5",
    "expected_temperature": "21",
    "label": "Bathroom",
    "timestamp": "2024-10-06 10:30:01"
  },
  "pipe": {
    "temperature": "37.25",
    "timestamp": "2024-10-06 10:30:01",
    "label": "Heating Pipe"
  }
}
```

### Accessing the database console

Install `sudo apt install -y sqlite` and run:

```
sqlite3 /srv/neuroheat/neuroheat.db
```

### Accessing logs

```
journalctl --unit neuroheat.service --lines=50 --follow
```

### Incompatible changes in the database schema

This is a hobby project, so it does not implement any migration mechanism (yet?). If there are changes in the schema (i.e., `src/repo.rs`'s init function), you may need to re-initialize the database:

1. Stop the service on the server:

    ```
    sudo systemctl stop neuroheat.service
    ```

1. Remove the database on the server:

    ```
    sudo rm /srv/neuroheat/neuroheat.db
    ```

1. Let the application initialize it from scratch on startup:

    ```
    ./bin/deploy
    ```

## Raspberry Pi Zero Setup

### On the memory card

1. Install [Raspbian Buster Lite](https://www.raspberrypi.org/downloads/raspbian/) on the memory card. See [official documentation](https://www.raspberrypi.org/documentation/installation/installing-images/README.md) for details.
1. Copy `server/config.txt` onto the card (as `config.txt`).
1. Configure the network connection by creating a `wpa_supplicant.conf` file on the card with the following content (adjust the credentials):

    ```
    country=PL
    ctrl_interface=DIR=/var/run/wpa_supplicant GROUP=netdev
    update_config=1
    network={
        ssid="[wifi-name]"
        psk="[wifi-password]"
        key_mgmt=WPA-PSK
    }
    ```
1. Enable SSH by creating an `ssh` file.

### From the host system after booting the server:

1. [Allow password-less SSH connections](https://www.raspberrypi.org/documentation/remote-access/ssh/passwordless.md):

    ```
    ssh-copy-id pi@[ip]
    ```

### On the server

1. [Set the hostname](https://thepihut.com/blogs/raspberry-pi-tutorials/19668676-renaming-your-raspberry-pi-the-hostname):

    ```
    sudo sed -i 's/raspberrypi/[neuroheat or any other hostname]/g' /etc/hostname
    sudo sed -i 's/raspberrypi/[neuroheat or any other hostname]/g' /etc/hosts
    ```

1. [Fix the `cannot change locale (en_US.UTF-8)` issue](https://www.jaredwolff.com/raspberry-pi-setting-your-locale/):

    ```
    sudo sed -i 's/# en_US.UTF-8 UTF-8/en_US.UTF-8 UTF-8/g' /etc/locale.gen
    sudo locale-gen en_US.UTF-8
    sudo update-locale en_US.UTF-8
    ```

1. Set the new password:

    ```
    sudo passwd pi
    ```

1. Upgrade the distro:

    ```
    sudo apt-get update
    sudo apt-get dist-upgrade
    sudo apt-get autoclean
    ```

1. Set the local timezone:

    ```
    sudo timedatectl set-timezone Europe/Warsaw
    ```

1. Configure [rsyslog](https://www.rsyslog.com/doc/master/tutorials/reliable_forwarding.html) (optional):

    ```
    # /etc/rsyslog.d/01-ignore-rngd.conf
    if $programname == 'rngd' then /var/log/rngd.log
    & stop

    if $programname == 'rng-tools' then /var/log/rngd.log
    & stop

    # /etc/rsyslog.d/02-cron.conf
    if $programname == 'cron' then /var/log/cron.log
    & stop

    # /etc/rsyslog.d/99-nas.conf
    use local address like "rsyslog.local"
    *.* @[ip address of log server]:514

    $ActionQueueFileName queue
    $ActionQueueMaxDiskSpace 1g
    $ActionQueueSaveOnShutdown on
    $ActionQueueType LinkedList
    $ActionResumeRetryCount -1

    # /etc/logrotate.d/rsyslog
    /var/log/rngd.log
    {
      rotate 4
      weekly
      missingok
      notifempty
      compress
      delaycompress
      sharedscripts
      postrotate
        /usr/lib/rsyslog/rsyslog-rotate
      endscript
    ```

1. Create application directories:

    ```
    mkdir -p /srv/backups /srv/neuroheat /opt/neuroheat/bin
    chown -R pi:pi /opt/neuroheat/ /srv/backups/
    ```

1. Configure backups:

    ```
    # Add in crontab:

    0 * * * * cp /srv/neuroheat/neuroheat.db /srv/backups/neuroheat.db.$(date +'%Y.%m.%d.%H.%M').bak
    15 10 * * * find /srv/backups -type f -mtime +14 -ls -exec rm -f -- {} \;
    ```

### From the host system after setting up the server:

1. Deploy the code:

    ```
    ./bin/deploy
    ```
