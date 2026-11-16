#!/bin/sh
# The YO3 Revival monitor (#37), run every minute by yo3-monitor.timer: checks the server's health endpoint, its disk
# and its backups, and sends an alert when a problem starts and again when it ends (never twice for one problem).
# Configuration: /etc/yo3/alerts.env (alerts.env.example). Needs curl. docs/hosting.md, "Monitoring and alerts".
set -u
health_url=${YO3_HEALTH_URL:-http://127.0.0.1:8080/health}
data_dir=${YO3_DATA_DIR:-/var/lib/yo3}
state=${YO3_MONITOR_STATE:-${STATE_DIRECTORY:-/var/lib/yo3-monitor}}
offsite_stamp=${YO3_OFFSITE_STAMP:-/var/lib/yo3-offsite/last-success}
backup_max=${YO3_BACKUP_MAX_AGE:-7200}
offsite_max=${YO3_OFFSITE_MAX_AGE:-93600}
disk_max=${YO3_DISK_MAX_PERCENT:-90}
errors_max=${YO3_ERRORS_MAX:-5}
host=$(hostname 2>/dev/null || echo server)
mkdir -p "$state"
sent=0

# Sends one message to the journal and to the configured channels.
alert() {
    message="[yo3 $host] $1"
    echo "$message"
    sent=1
    if [ -n "${YO3_ALERT_WEBHOOK:-}" ]; then
        escaped=$(printf '%s' "$message" | sed 's/\\/\\\\/g; s/"/\\"/g')
        curl -fsS -m 10 -H 'Content-Type: application/json' -d "{\"content\":\"$escaped\"}" "$YO3_ALERT_WEBHOOK" >/dev/null \
            || echo "the alert above was not delivered to the webhook"
    fi
    if [ -n "${YO3_ALERT_COMMAND:-}" ]; then
        # shellcheck disable=SC2086 # the command may carry its own arguments
        $YO3_ALERT_COMMAND "$message" || echo "the alert command failed"
    fi
}

# check NAME FAILING TEXT: alerts when NAME starts failing (FAILING is 1) and when it is over.
check() {
    if [ "$2" = 1 ]; then
        [ -e "$state/$1" ] || { alert "PROBLEM: $3"; : > "$state/$1"; }
    elif [ -e "$state/$1" ]; then
        rm -f "$state/$1"
        alert "RESOLVED: $1"
    fi
}

# A JSON field's bare value from the health answer (true, false or a number).
field() { printf '%s' "$health" | sed -n "s/.*\"$1\": *\([a-z0-9.]*\).*/\1/p"; }

health=$(curl -sS -m 10 "$health_url" 2>/dev/null)
if [ -z "$health" ]; then
    check server-down 1 "the server doesn't answer its health check ($health_url). systemd restarts a failed server: it stayed down."
else
    check server-down 0 ""
    [ "$(field saving)" = false ] && f=1 || f=0
    check saving-fails "$f" "the server can't save (a full disk or a broken database): logins are paused until it can."
    [ "$(field engine)" = false ] && f=1 || f=0
    check duels-broken "$f" "the rules engine can't start: duels end at once as draws."
    age=$(field backup_age_seconds)
    if [ -z "$age" ] || [ "$age" -gt "$backup_max" ]; then f=1; else f=0; fi
    check backup-stale "$f" "the newest backup is ${age:-missing}${age:+ seconds old} (limit $backup_max s)."
    errors=$(field errors_recent)
    [ "${errors:-0}" -ge "$errors_max" ] && f=1 || f=0
    check errors-repeat "$f" "$errors errors in the server's log in 10 minutes: see the admin panel's Logs page."
fi

used=$(df -P "$data_dir" 2>/dev/null | awk 'NR==2 { sub("%", "", $5); print $5 }')
[ -n "$used" ] && [ "$used" -ge "$disk_max" ] && f=1 || f=0
check disk-filling "$f" "the data folder's disk is ${used:-?}% full (limit $disk_max%)."

# Only once an off-machine backup has run: its stamp holds the time of the last success.
if [ -f "$offsite_stamp" ]; then
    last=$(cat "$offsite_stamp" 2>/dev/null)
    now=$(date +%s)
    [ $((now - ${last:-0})) -gt "$offsite_max" ] && f=1 || f=0
    check offsite-stale "$f" "the last off-machine backup is over $((offsite_max / 3600)) hours old."
fi

[ "$sent" = 0 ] && echo "all clear"
exit 0
