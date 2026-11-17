#!/bin/sh
# Stage only the server's runtime files, using libraries from the SAME image that compiled the binaries.
set -eu
out=$1
root=$2
install -d -m 0755 "$root/opt/yo3" "$root/etc/yo3" "$root/etc/ssl/certs" "$root/usr/local/bin"
cp -a "$out/." "$root/opt/yo3/"
install -d -m 1777 "$root/tmp"
install -d -o 10001 -g 10001 -m 0700 "$root/var/lib/yo3"
install -d -o 10001 -g 10001 -m 0750 "$root/run/yo3-dashboard"
printf 'yo3:x:10001:10001:YO3 server:/var/lib/yo3:/nonexistent\n' > "$root/etc/passwd"
printf 'yo3:x:10001:\n' > "$root/etc/group"
printf 'passwd: files\ngroup: files\nhosts: files dns\n' > "$root/etc/nsswitch.conf"
cp /etc/ssl/certs/ca-certificates.crt "$root/etc/ssl/certs/"
cp /usr/share/zoneinfo/Etc/UTC "$root/etc/localtime"
cp -L /etc/os-release "$root/etc/os-release"

# Preserve absolute paths, including the ELF interpreter. Do not flatten libraries or rely on LD_LIBRARY_PATH.
# Only known trusted executables produced/installed in this builder are passed to ldd.
packages=$(mktemp)
trap 'rm -f "$packages"' EXIT
copy_runtime_file() {
    cp --parents -L "$1" "$root"
    LC_ALL=C dpkg-query -S "$1" 2>/dev/null | sed -n '/diversion /d; s/: \/.*//p' >> "$packages" || true
}
for binary in "$out/yo3-server" "$out/yo3-engine" /usr/bin/curl /bin/sh /bin/cat /usr/bin/id; do
    libraries=$(LC_ALL=C ldd "$binary")
    if printf '%s\n' "$libraries" | grep -q 'not found'; then
        echo "Missing runtime library for $binary" >&2
        exit 1
    fi
    printf '%s\n' "$libraries" | awk '$2 == "=>" && $3 ~ /^\// {print $3} $1 ~ /^\// {print $1}' | sort -u |
        while IFS= read -r library; do copy_runtime_file "$library"; done
    case "$binary" in /usr/bin/*|/bin/*) copy_runtime_file "$binary";; esac
done
# glibc may load NSS modules at runtime; these are not reported by ldd. Keep files/DNS resolution functional.
for module in /lib/*/libnss_dns.so.2 /lib/*/libnss_files.so.2; do
    [ ! -f "$module" ] || copy_runtime_file "$module"
done
printf '%s\n' ca-certificates tzdata >> "$packages"
sort -u "$packages" -o "$packages"
install -d "$root/usr/share/doc/yo3-runtime"
install -d "$root/var/lib/dpkg"
while IFS= read -r package; do
    name=${package%%:*}
    if [ -f "/usr/share/doc/$name/copyright" ]; then
        install -d "$root/usr/share/doc/$name"
        cp -L "/usr/share/doc/$name/copyright" "$root/usr/share/doc/$name/"
    fi
    # Retain package identity for vulnerability scanners, without installing a package manager in the runtime.
    dpkg-query -s "$package" >> "$root/var/lib/dpkg/status"
    printf '\n' >> "$root/var/lib/dpkg/status"
    dpkg-query -W -f '${binary:Package}\t${Version}\t${source:Package}\t${source:Version}\n' "$package"
done < "$packages" > "$root/usr/share/doc/yo3-runtime/packages.tsv"
