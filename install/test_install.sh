#!/bin/sh
# install.sh saf-sh birim testleri (bağımlılık yok, ağ yok).
#
#   sh install/test_install.sh
#
# install.sh'in saf yardımcılarını kaynaklar; akış çalışmaz
# (ACTOS_INSTALL_SOURCED guard'ı sayesinde).

ACTOS_INSTALL_SOURCED=1
# shellcheck disable=SC1091
. "$(dirname "$0")/install.sh"

PASS=0
FAIL=0

ok() { PASS=$((PASS + 1)); }
fail() { FAIL=$((FAIL + 1)); printf 'FAIL %s\n  %s\n' "$1" "$2"; }

assert_true() { # $1=name $2... = command
    _name="$1"; shift
    if "$@" >/dev/null 2>&1; then ok; else fail "$_name" "expected success: $*"; fi
}

assert_false() { # $1=name $2... = command
    _name="$1"; shift
    if "$@" >/dev/null 2>&1; then fail "$_name" "expected failure: $*"; else ok; fi
}

assert_eq() { # $1=name $2=expected $3=actual
    if [ "$2" = "$3" ]; then ok; else fail "$1" "expected [$2], got [$3]"; fi
}

# --- version_lt ---
assert_true  "lt basic"            version_lt 0.1.1 0.2.0
assert_false "lt equal"            version_lt 0.2.0 0.2.0
assert_false "lt greater"          version_lt 0.2.0 0.1.1
assert_true  "lt minor beats patch" version_lt 0.1.9 0.2.0
assert_true  "lt longer wins"      version_lt 0.2 0.2.1
assert_false "lt longer reverse"   version_lt 0.2.1 0.2
assert_true  "lt major"            version_lt 1.9.9 2.0.0
assert_true  "lt prerelease"       version_lt 1.0-beta 1.0.0
assert_false "lt release-vs-pre"   version_lt 1.0.0 1.0-beta
assert_false "lt leading zeros"    version_lt 0.10.0 0.9.0
assert_false "lt garbage equal"    version_lt abc abc

# --- parse_search_version ---
assert_eq "parse valid" \
    "0.2.0" \
    "$(parse_search_version 'actos-cli = "0.2.0" # Official command-line client')"
assert_eq "parse ignores others" \
    "" \
    "$(parse_search_version 'actos-cli-extra = "9.9.9" # nope')"
assert_eq "parse garbage" "" "$(parse_search_version 'nothing here')"

# --- decide_action ---
assert_eq "up-to-date" "up-to-date" "$(decide_action 0.2.0 0.2.0 0)"
assert_eq "upgrade"    "upgrade"    "$(decide_action 0.1.1 0.2.0 0)"
assert_eq "downgrade-is-install" "install" "$(decide_action 0.2.0 0.1.1 0)"
assert_eq "fresh install"  "install" "$(decide_action '' 0.2.0 0)"
assert_eq "force reinstall" "install" "$(decide_action 0.2.0 0.2.0 1)"
assert_eq "offline keep"   "unknown" "$(decide_action 0.1.1 '' 0)"
assert_eq "offline fresh"  "unknown" "$(decide_action '' '' 0)"

# --- installed_version (sahte ikili ile) ---
FAKEBIN="$(mktemp -d)/actos"
printf '#!/bin/sh\necho "actos 0.1.1"\n' > "$FAKEBIN"
chmod +x "$FAKEBIN"
assert_eq "installed_version" "0.1.1" "$(installed_version "$FAKEBIN")"

# --- latest_published (sahte cargo ile, ağ yok) ---
FAKEDIR="$(mktemp -d)"
printf '#!/bin/sh\necho arbitrarily >&2\necho \x27actos-cli = "0.2.0" # desc\x27\n' > "$FAKEDIR/cargo"
chmod +x "$FAKEDIR/cargo"
assert_eq "latest_published" \
    "0.2.0" \
    "$(PATH="$FAKEDIR:$PATH" latest_published)"

printf '\n%d passed, %d failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
