# Neuroheat

This is another version of Heating Brain hobby project - Elixir application for
controlling floor heating system in the house. It utilizes Raspberry PI Zero,
a relay controller, and few 1-wire temperature sensors. The original Elixir
application was running for over three years without any issues (except CPU
and memory utilization when rendering data). It gave around 20-30% savings
in gas consumption comparing to the previous "industry standard" devices
(mainly because optimized logic for enabling and disabling heating in context
of required heat area).

## Development

Install tools required by `.tool-versions` file and ensure you have the
following system packages installed on your macOC:

```
brew install arm-unknown-linux-musleabihf
rustup target add arm-unknown-linux-musleabihf
```

## Deployment

There is `bin/deploy` script that builds the binary file and performs actions
on the remote server (e.g., backing up database, updating systemd service, etc.).
Make sure to reving the heating configuration (e.g., GPIO pins, sensor
identifiers, etc.).

You will have to setup the server and create `heating_config.json` (see
`heating_config.json.sample`) if running of the first time.

## Raspberry Pi Zero Setup

### On the memory card

1. Install [Raspbian Buster Lite](https://www.raspberrypi.org/downloads/raspbian/) on the memory card. See [official documentation](https://www.raspberrypi.o
rg/documentation/installation/installing-images/README.md) for details.
1. Copy `server/config.txt` into the card (as `config.txt`).
1. Configure network connection by creating `wpa_supplicant.conf` file on the card with the following content (adjust the credentials):

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
1. Enable SSH by creating `ssh` file.

### From host system after booting the server:

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

1. Upgrade distro:

    ```
    sudo apt-get update
    sudo apt-get dist-upgrade
    sudo apt-get autoclean
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

### From host system after setting up the server:

1. Deploy the code:

    ```
    ./bin/deploy
    ```
