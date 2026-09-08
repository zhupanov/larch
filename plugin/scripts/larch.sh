#!/usr/bin/env bash
# Install the release-matched Rust executable when needed, then replace this
# process with it. Keep this bootstrap compatible with macOS Bash 3.2.
set -eEuo pipefail

readonly RELEASE_REPO="character-ai/larch"
readonly RELEASE_WORKFLOW="character-ai/larch/.github/workflows/rust-release-assets.yaml"
# The branch .claude-plugin/marketplace.json pins installed plugin content to.
# Shared with RELEASE_PIN_REF in crates/larch-cli/src/release_publish.rs and the
# descriptor's "ref" field; all three change together. Not "release": Git refs
# are paths, so refs/heads/release cannot coexist with the release/v<version>
# candidate branches /release Step 5 creates.
readonly RELEASE_PIN_REF="stable"
readonly LOCK_WAIT_SECONDS=120
readonly LARCH_NO_INSTALL_EXIT=97

plugin_root=""
plugin_data=""
bin_dir=""
binary_path=""
stage_dir=""
stage_parent=""
lock_dir=""
lock_held=false
sha_command=""

retry_hint() {
    printf '%s\n' "Retry the command after correcting the reported problem. No existing larch binary was changed before verified installation." >&2
}

die() {
    printf 'larch bootstrap: %s\n' "$1" >&2
    retry_hint
    exit 1
}

read_lock_owner() {
    local owner_path="$1"
    local owner=""
    [ -f "$owner_path" ] && [ ! -L "$owner_path" ] || return 1
    owner="$(awk 'NR == 1 && $0 ~ /^[1-9][0-9]*$/ { value = $0; next } { exit 1 } END { if (NR == 1) print value; else exit 1 }' "$owner_path")" || return 1
    printf '%s\n' "$owner"
}

cleanup() {
    local owner=""
    if [ "$lock_held" = true ] && [ -n "$lock_dir" ] && [ -d "$lock_dir" ] && [ ! -L "$lock_dir" ]; then
        owner="$(read_lock_owner "$lock_dir/owner" 2>/dev/null || true)"
        if [ "$owner" = "$$" ]; then
            rm -f -- "$lock_dir/owner"
            rmdir "$lock_dir" 2>/dev/null || true
        fi
    fi
    lock_held=false
    if [ -n "$stage_dir" ] && [ -n "$stage_parent" ]; then
        case "$stage_dir" in
            "$stage_parent"/.larch-bootstrap.*)
                if [ -d "$stage_dir" ] && [ ! -L "$stage_dir" ]; then
                    rm -rf -- "$stage_dir"
                fi
                ;;
        esac
    fi
    stage_dir=""
}

unexpected_error() {
    local status="$1"
    local line="$2"
    trap - ERR
    printf 'larch bootstrap: installation failed at script line %s (exit %s).\n' "$line" "$status" >&2
    retry_hint
    exit "$status"
}

trap cleanup EXIT
trap 'unexpected_error "$?" "$LINENO"' ERR

validate_absolute_path() {
    local label="$1"
    local path="$2"
    local probe=""
    case "$path" in
        /*) ;;
        *) die "$label must be an absolute path" ;;
    esac
    case "$path/" in
        *$'\n'*|*'/../'*|*'/./'*|*'//'*) die "$label contains an unsafe path component" ;;
    esac
    probe="$path"
    while [ "$probe" != "/" ]; do
        if [ -L "$probe" ]; then
            die "$label or one of its existing ancestors is a symlink: $probe"
        fi
        probe="${probe%/*}"
        [ -n "$probe" ] || probe="/"
    done
}

read_plugin_version() {
    local manifest="$1"
    local version=""
    [ -f "$manifest" ] && [ ! -L "$manifest" ] || die "plugin manifest is missing or unsafe: $manifest"
    version="$(awk -F '"' '$2 == "version" { count++; value = $4 } END { if (count == 1) print value; else exit 1 }' "$manifest")" || die "plugin manifest must contain exactly one version string"
    if ! [[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
        die "plugin manifest version is not a release semantic version"
    fi
    printf '%s\n' "$version"
}

validate_release_version() {
    local version="$1"
    if ! [[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
        die "requested release version is not a semantic version"
    fi
}

resolve_target() {
    local os_name=""
    local architecture=""
    os_name="$(uname -s)"
    architecture="$(uname -m)"
    case "$os_name:$architecture" in
        Darwin:arm64|Darwin:aarch64) printf '%s\n' "aarch64-apple-darwin" ;;
        Darwin:x86_64|Darwin:amd64) printf '%s\n' "x86_64-apple-darwin" ;;
        Linux:arm64|Linux:aarch64) printf '%s\n' "aarch64-unknown-linux-gnu" ;;
        Linux:x86_64|Linux:amd64) printf '%s\n' "x86_64-unknown-linux-gnu" ;;
        *) die "unsupported operating system or architecture: $os_name/$architecture" ;;
    esac
}

require_release_install_target() {
    local target="$1"
    case "$target" in
        aarch64-apple-darwin) ;;
        *) die "unsupported operating system or architecture for release install: $target (larch releases ship only aarch64-apple-darwin)" ;;
    esac
}

binary_matches_version() {
    local candidate="$1"
    local expected_version="$2"
    local reported=""
    [ -f "$candidate" ] && [ ! -L "$candidate" ] && [ -x "$candidate" ] || return 1
    reported="$("$candidate" --version 2>/dev/null)" || return 1
    [ "$reported" = "larch $expected_version" ]
}

binary_passes_self_check() {
    local candidate="$1"
    local expected_version="$2"
    local expected_target="$3"
    local reported=""
    local expected=""
    reported="$("$candidate" bootstrap self-check 2>/dev/null)" || return 1
    expected="{\"schema_version\":1,\"version\":\"$expected_version\",\"target\":\"$expected_target\"}"
    [ "$reported" = "$expected" ]
}

require_commands() {
    local command_name=""
    for command_name in awk chmod cmp dd gh gzip kill ln mkdir mktemp mv rm rmdir sed sleep sort tar tr uname wc; do
        command -v "$command_name" >/dev/null 2>&1 || die "required tool is missing: $command_name"
    done
    if command -v sha256sum >/dev/null 2>&1; then
        sha_command="sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
        sha_command="shasum"
    else
        die "required SHA-256 tool is missing: install sha256sum or shasum"
    fi
    gh release verify --help >/dev/null 2>&1 || die "GitHub CLI is too old: gh release verify is required"
    gh attestation verify --help >/dev/null 2>&1 || die "GitHub CLI is too old: gh attestation verify is required"
    gh release view --help 2>/dev/null | awk '/isImmutable/ { found = 1 } END { exit(found ? 0 : 1) }' || die "GitHub CLI is too old: immutable release metadata is required"
}

sha256_file() {
    local path="$1"
    local digest=""
    if [ "$sha_command" = "sha256sum" ]; then
        digest="$(sha256sum "$path" | awk '{ print $1 }')"
    else
        digest="$(shasum -a 256 "$path" | awk '{ print $1 }')"
    fi
    case "$digest" in
        *[!0-9a-f]*|'') die "SHA-256 tool returned an invalid digest for ${path##*/}" ;;
    esac
    [ "${#digest}" -eq 64 ] || die "SHA-256 tool returned an invalid digest length for ${path##*/}"
    printf '%s\n' "$digest"
}

acquire_lock() {
    local attempts=0
    local observed_owner=""
    local moved_owner=""
    local stale_claim=""
    mkdir -p -- "$plugin_data"
    [ -d "$plugin_data" ] && [ ! -L "$plugin_data" ] || die "plugin data root is not a real directory"
    chmod 700 "$plugin_data"
    lock_dir="$plugin_data/bootstrap.lock"
    while ! mkdir "$lock_dir" 2>/dev/null; do
        [ -d "$lock_dir" ] && [ ! -L "$lock_dir" ] || die "bootstrap lock path is unsafe"
        observed_owner="$(read_lock_owner "$lock_dir/owner" 2>/dev/null || true)"
        if [ -n "$observed_owner" ] && ! kill -0 "$observed_owner" 2>/dev/null; then
            stale_claim="$plugin_data/.bootstrap-lock-stale.$$"
            [ ! -e "$stale_claim" ] && [ ! -L "$stale_claim" ] || die "stale-lock claim path already exists"
            if mv "$lock_dir" "$stale_claim" 2>/dev/null; then
                moved_owner="$(read_lock_owner "$stale_claim/owner" 2>/dev/null || true)"
                if [ "$moved_owner" != "$observed_owner" ]; then
                    [ -e "$lock_dir" ] || mv "$stale_claim" "$lock_dir"
                else
                    rm -f -- "$stale_claim/owner"
                    if ! rmdir "$stale_claim"; then
                        [ -e "$lock_dir" ] || mv "$stale_claim" "$lock_dir"
                        die "stale bootstrap lock contains unexpected files"
                    fi
                fi
            fi
        elif [ -z "$observed_owner" ] && [ "$attempts" -ge 2 ]; then
            rmdir "$lock_dir" 2>/dev/null || true
        fi
        attempts=$((attempts + 1))
        [ "$attempts" -lt "$LOCK_WAIT_SECONDS" ] || die "timed out waiting for another larch bootstrap; retry after it exits"
        sleep 1
    done
    lock_held=true
    umask 077
    printf '%s\n' "$$" > "$lock_dir/owner"
    [ "$(read_lock_owner "$lock_dir/owner")" = "$$" ] || die "bootstrap lock ownership could not be verified"
}

write_expected_release_assets() {
    local version="$1"
    local output="$2"
    printf '%s\n' \
        "larch-v$version-SHA256SUMS" \
        "larch-v$version-aarch64-apple-darwin.tar.gz" \
        "larch-v$version-manifest.json" | sort > "$output"
}

# Call from a function body, never inside a command substitution: `die` must run
# in the main shell so one clear message replaces the generic ERR-trap report.
require_commit_sha() {
    local label="$1"
    local sha="$2"
    case "$sha" in
        *[!0-9a-f]*|'') die "$label is invalid" ;;
    esac
    [ "${#sha}" -eq 40 ] || die "$label has an invalid length"
}

# Prove the plugin content a pinned install would fetch and the binary this
# release ships come from one commit, before any plugin state is mutated.
verify_release_pin() {
    local tag="$1"
    local tag_commit="$2"
    local pin_commit=""
    pin_commit="$(gh api "repos/$RELEASE_REPO/git/ref/heads/$RELEASE_PIN_REF" --jq '.object.sha' 2>/dev/null || true)"
    [ -n "$pin_commit" ] || die "release pin branch refs/heads/$RELEASE_PIN_REF could not be read; the marketplace descriptor pins installed plugin content to it"
    require_commit_sha "release pin commit" "$pin_commit"
    if [ "$pin_commit" != "$tag_commit" ]; then
        die "plugin content and executable would come from different commits: refs/heads/$RELEASE_PIN_REF is at $pin_commit but $tag is at $tag_commit. Retry after the in-flight release finishes advancing the pin."
    fi
    printf 'LARCH_PREFLIGHT_PIN_VERIFIED=true\n'
}

verify_release_surface() {
    local version="$1"
    local tag="$2"
    local release_info="$stage_dir/release-info.txt"
    local actual_assets="$stage_dir/release-assets.txt"
    local expected_assets="$stage_dir/expected-assets.txt"
    gh release verify "$tag" --repo "$RELEASE_REPO" >/dev/null
    gh release view "$tag" --repo "$RELEASE_REPO" \
        --json tagName,isImmutable,isDraft,isPrerelease,assets \
        --jq '.tagName, (.isImmutable | tostring), (.isDraft | tostring), (.isPrerelease | tostring), (.assets[].name)' > "$release_info"
    [ "$(sed -n '1p' "$release_info")" = "$tag" ] || die "release tag identity mismatch"
    [ "$(sed -n '2p' "$release_info")" = "true" ] || die "release is not immutable"
    [ "$(sed -n '3p' "$release_info")" = "false" ] || die "release is still a draft"
    [ "$(sed -n '4p' "$release_info")" = "false" ] || die "release is a prerelease"
    sed -n '5,$p' "$release_info" | sort > "$actual_assets"
    write_expected_release_assets "$version" "$expected_assets"
    cmp -s "$actual_assets" "$expected_assets" || die "release asset allowlist mismatch"
}

validate_checksums() {
    local path="$1"
    local version="$2"
    local target="$3"
    awk -v version="$version" -v selected="$target" '
        function fail() { exit 1 }
        {
            if (NR > 2 || length($0) < 67) fail()
            digest = substr($0, 1, 64)
            separator = substr($0, 65, 2)
            name = substr($0, 67)
            if (digest !~ /^[0-9a-f]+$/ || length(digest) != 64 || separator != "  ") fail()
            if (NR == 1) {
                target = "aarch64-apple-darwin"
                expected = "larch-v" version "-" target ".tar.gz"
                if (name != expected) fail()
                if (target == selected) selected_digest = digest
            } else {
                if (name != "larch-v" version "-manifest.json") fail()
                manifest_digest = digest
            }
        }
        END {
            if (NR != 2 || selected_digest == "" || manifest_digest == "") exit 1
            print manifest_digest, selected_digest
        }
    ' "$path"
}

validate_manifest() {
    local path="$1"
    local version="$2"
    local tag="$3"
    local source_commit="$4"
    local selected_target="$5"
    awk -v version="$version" -v tag="$tag" -v commit="$source_commit" -v selected="$selected_target" '
        function fail() { exit 1 }
        function exact(expected) { if ($0 != expected) fail() }
        NR == 1 { exact("{"); next }
        NR == 2 { exact("  \"schema_version\": 1,"); next }
        NR == 3 { exact("  \"plugin_version\": \"" version "\","); next }
        NR == 4 { exact("  \"tag\": \"" tag "\","); next }
        NR == 5 { exact("  \"source_commit\": \"" commit "\","); next }
        NR == 6 { exact("  \"assets\": ["); next }
        NR >= 7 && NR <= 17 {
            position = NR - 6
            target = "aarch64-apple-darwin"
            if (position == 1) exact("    {")
            else if (position == 2) exact("      \"target\": \"" target "\",")
            else if (position == 3) exact("      \"archive\": \"larch-v" version "-" target ".tar.gz\",")
            else if (position == 4) {
                if ($0 !~ /^      "byte_size": [1-9][0-9]*,$/) fail()
                value = $0
                sub(/^      "byte_size": /, "", value)
                sub(/,$/, "", value)
                if (target == selected) selected_size = value
            }
            else if (position == 5) {
                if ($0 !~ /^      "sha256": "[0-9a-f]+",$/) fail()
                value = $0
                sub(/^      "sha256": "/, "", value)
                sub(/",$/, "", value)
                if (length(value) != 64) fail()
                if (target == selected) selected_digest = value
            }
            else if (position == 6) exact("      \"binary_path\": \"larch\",")
            else if (position == 7) exact("      \"minimum_os_or_libc\": {")
            else if (position == 8) exact("        \"kind\": \"macos\",")
            else if (position == 9) exact("        \"version\": \"11.0\"")
            else if (position == 10) exact("      }")
            else exact("    }")
            next
        }
        NR == 18 { exact("  ]"); next }
        NR == 19 { exact("}"); next }
        { fail() }
        END {
            if (NR != 19 || selected_size == "" || selected_digest == "") exit 1
            print selected_size, selected_digest
        }
    ' "$path"
}

header_text() {
    local tar_path="$1"
    local offset="$2"
    local length="$3"
    dd if="$tar_path" bs=1 skip="$offset" count="$length" 2>/dev/null | tr -d '\000'
}

header_octal_size() {
    local tar_path="$1"
    local header_offset="$2"
    local raw=""
    raw="$(dd if="$tar_path" bs=1 skip="$((header_offset + 124))" count=12 2>/dev/null | tr -d '\000 ')"
    case "$raw" in
        *[!0-7]*|'') die "archive contains an invalid tar size field" ;;
    esac
    printf '%s\n' "$((8#$raw))"
}

validate_tar_header() {
    local tar_path="$1"
    local header_offset="$2"
    local expected_name="$3"
    [ "$(header_text "$tar_path" "$header_offset" 100)" = "$expected_name" ] || die "archive member name allowlist mismatch"
    [ "$(header_text "$tar_path" "$((header_offset + 156))" 1)" = "0" ] || die "archive contains a symlink or special file"
    [ -z "$(header_text "$tar_path" "$((header_offset + 157))" 100)" ] || die "archive member has an unexpected link target"
    [ -z "$(header_text "$tar_path" "$((header_offset + 345))" 155)" ] || die "archive member has an unexpected path prefix"
}

validate_and_extract_archive() {
    local archive="$1"
    local output_binary="$2"
    local tar_path="$stage_dir/archive.tar"
    local members="$stage_dir/archive-members.txt"
    local expected_members="$stage_dir/expected-members.txt"
    local first_size=0
    local second_offset=0
    local second_size=0
    local trailer_offset=0
    local tar_size=0
    local nonzero_trailer=0
    gzip -dc "$archive" > "$tar_path"
    tar -tzf "$archive" > "$members"
    printf 'larch\nLICENSE\n' > "$expected_members"
    cmp -s "$members" "$expected_members" || die "archive member allowlist mismatch"
    validate_tar_header "$tar_path" 0 "larch"
    first_size="$(header_octal_size "$tar_path" 0)"
    [ "$first_size" -gt 0 ] || die "archive executable is empty"
    second_offset=$((512 + ((first_size + 511) / 512) * 512))
    validate_tar_header "$tar_path" "$second_offset" "LICENSE"
    second_size="$(header_octal_size "$tar_path" "$second_offset")"
    [ "$second_size" -gt 0 ] || die "archive license is empty"
    trailer_offset=$((second_offset + 512 + ((second_size + 511) / 512) * 512))
    tar_size="$(wc -c < "$tar_path" | tr -d '[:space:]')"
    [ "$tar_size" -ge $((trailer_offset + 1024)) ] || die "archive is missing the tar end marker"
    [ $((tar_size % 512)) -eq 0 ] || die "archive tar size is not block aligned"
    nonzero_trailer="$(dd if="$tar_path" bs=1 skip="$trailer_offset" 2>/dev/null | tr -d '\000' | wc -c | tr -d '[:space:]')"
    [ "$nonzero_trailer" -eq 0 ] || die "archive contains unexpected trailing data"
    umask 077
    tar -xOzf "$archive" larch > "$output_binary"
    chmod 755 "$output_binary"
    [ -f "$output_binary" ] && [ ! -L "$output_binary" ] && [ -x "$output_binary" ] || die "staged executable is not a regular executable file"
}

verify_download_set() {
    local download_dir="$1"
    local manifest_name="$2"
    local checksums_name="$3"
    local archive_name="$4"
    local entry=""
    local name=""
    local count=0
    for entry in "$download_dir"/* "$download_dir"/.[!.]* "$download_dir"/..?*; do
        if [ ! -e "$entry" ] && [ ! -L "$entry" ]; then
            continue
        fi
        count=$((count + 1))
        [ -f "$entry" ] && [ ! -L "$entry" ] || die "download staging contains a non-regular entry"
        name="${entry##*/}"
        case "$name" in
            "$manifest_name"|"$checksums_name"|"$archive_name") ;;
            *) die "download staging contains an unexpected asset: $name" ;;
        esac
    done
    [ "$count" -eq 3 ] || die "download staging does not contain exactly three requested assets"
}

install_release_binary() {
    local version="$1"
    local target="$2"
    local publish_binary="${3:-true}"
    local verify_pin="${4:-false}"
    local tag="v$version"
    local source_commit=""
    local download_dir=""
    local manifest_name="larch-v$version-manifest.json"
    local checksums_name="larch-v$version-SHA256SUMS"
    local archive_name="larch-v$version-$target.tar.gz"
    local manifest_path=""
    local checksums_path=""
    local archive_path=""
    local checksum_record=""
    local manifest_record=""
    local manifest_checksum=""
    local archive_checksum=""
    local archive_size=""
    local archive_manifest_digest=""
    local actual_size=""
    local actual_digest=""
    local staged_binary=""
    local previous_binary=""

    require_release_install_target "$target"
    verify_release_surface "$version" "$tag"
    source_commit="$(gh api "repos/$RELEASE_REPO/commits/$tag" --jq '.sha')"
    require_commit_sha "release source commit" "$source_commit"
    if [ "$verify_pin" = true ]; then
        verify_release_pin "$tag" "$source_commit"
    fi

    download_dir="$stage_dir/download"
    mkdir "$download_dir"
    gh release download "$tag" --repo "$RELEASE_REPO" --dir "$download_dir" \
        --pattern "$manifest_name" --pattern "$checksums_name" --pattern "$archive_name"
    verify_download_set "$download_dir" "$manifest_name" "$checksums_name" "$archive_name"
    manifest_path="$download_dir/$manifest_name"
    checksums_path="$download_dir/$checksums_name"
    archive_path="$download_dir/$archive_name"

    gh attestation verify "$manifest_path" --repo "$RELEASE_REPO" --signer-workflow "$RELEASE_WORKFLOW" \
        --source-ref "refs/tags/$tag" --source-digest "$source_commit" --deny-self-hosted-runners >/dev/null
    gh attestation verify "$checksums_path" --repo "$RELEASE_REPO" --signer-workflow "$RELEASE_WORKFLOW" \
        --source-ref "refs/tags/$tag" --source-digest "$source_commit" --deny-self-hosted-runners >/dev/null
    gh attestation verify "$archive_path" --repo "$RELEASE_REPO" --signer-workflow "$RELEASE_WORKFLOW" \
        --source-ref "refs/tags/$tag" --source-digest "$source_commit" --deny-self-hosted-runners >/dev/null

    if ! checksum_record="$(validate_checksums "$checksums_path" "$version" "$target")"; then
        die "checksum file violates the release schema"
    fi
    case "$checksum_record" in
        *' '*) ;;
        *) die "checksum parser returned malformed data" ;;
    esac
    manifest_checksum="${checksum_record%% *}"
    archive_checksum="${checksum_record#* }"
    case "$archive_checksum" in
        *' '*) die "checksum parser returned malformed data" ;;
    esac
    [ "$(sha256_file "$manifest_path")" = "$manifest_checksum" ] || die "manifest SHA-256 digest mismatch"

    if ! manifest_record="$(validate_manifest "$manifest_path" "$version" "$tag" "$source_commit" "$target")"; then
        die "manifest violates the strict schema or release identity"
    fi
    case "$manifest_record" in
        *' '*) ;;
        *) die "manifest parser returned malformed data" ;;
    esac
    archive_size="${manifest_record%% *}"
    archive_manifest_digest="${manifest_record#* }"
    case "$archive_manifest_digest" in
        *' '*) die "manifest parser returned malformed data" ;;
    esac
    [ "$archive_manifest_digest" = "$archive_checksum" ] || die "archive digest differs between manifest and checksum file"
    actual_size="$(wc -c < "$archive_path" | tr -d '[:space:]')"
    [ "$actual_size" = "$archive_size" ] || die "archive byte size does not match the manifest"
    actual_digest="$(sha256_file "$archive_path")"
    [ "$actual_digest" = "$archive_manifest_digest" ] || die "archive SHA-256 digest mismatch"

    staged_binary="$stage_dir/larch.staged"
    validate_and_extract_archive "$archive_path" "$staged_binary"
    binary_matches_version "$staged_binary" "$version" || die "staged executable reports the wrong version"
    binary_passes_self_check "$staged_binary" "$version" "$target" || die "staged executable failed its machine-readable self-check"

    if [ "$publish_binary" = false ]; then
        return 0
    fi

    if [ -e "$binary_path" ] || [ -L "$binary_path" ]; then
        [ -f "$binary_path" ] && [ ! -L "$binary_path" ] || die "existing larch binary is not a regular file"
        previous_binary="$stage_dir/larch.previous"
        ln "$binary_path" "$previous_binary"
    fi
    mv -f "$staged_binary" "$binary_path"
    if ! binary_matches_version "$binary_path" "$version" || ! binary_passes_self_check "$binary_path" "$version" "$target"; then
        if [ -n "$previous_binary" ] && [ -f "$previous_binary" ]; then
            mv -f "$previous_binary" "$binary_path"
        fi
        die "installed executable failed post-install verification"
    fi
}

preflight_release() {
    local version="$1"
    local target=""

    validate_release_version "$version"
    plugin_data="${CLAUDE_PLUGIN_DATA:-}"
    [ -n "$plugin_data" ] || die "CLAUDE_PLUGIN_DATA is required for release preflight staging"
    while [ "$plugin_data" != "/" ] && [ "${plugin_data%/}" != "$plugin_data" ]; do
        plugin_data="${plugin_data%/}"
    done
    validate_absolute_path "CLAUDE_PLUGIN_DATA" "$plugin_data"
    require_commands
    acquire_lock
    target="$(resolve_target)"
    stage_parent="$plugin_data"
    stage_dir="$(mktemp -d "$stage_parent/.larch-bootstrap.XXXXXX")"
    case "$stage_dir" in
        "$stage_parent"/.larch-bootstrap.*) ;;
        *) die "mktemp returned a preflight path outside plugin data" ;;
    esac
    [ -d "$stage_dir" ] && [ ! -L "$stage_dir" ] || die "release preflight staging path is unsafe"
    install_release_binary "$version" "$target" false true
    printf 'LARCH_PREFLIGHT_VERSION=%s\n' "$version"
}

latest_stable_version() {
    local version=""
    command -v gh >/dev/null 2>&1 || die "required tool is missing: gh"
    gh api --help >/dev/null 2>&1 || die "GitHub CLI is too old: gh api is required"
    # This repository exceeds GitHub's 1,000-result pagination ceiling. The
    # dedicated endpoint is bounded and excludes drafts and prereleases.
    version="$(
        gh api "repos/$RELEASE_REPO/releases/latest" --jq '.tag_name' |
            awk '{ sub(/^v/, ""); if (found == "" && $0 ~ /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/) found = $0 } END { if (found != "") print found }'
    )"
    [ -n "$version" ] || die "GitHub returned no valid stable larch release tags"
    validate_release_version "$version"
    printf 'LARCH_STABLE_VERSION=%s\n' "$version"
}

run_binary() {
    local candidate="$1"
    shift
    cleanup
    trap - EXIT ERR
    exec "$candidate" "$@"
    die "verified larch executable could not be started"
}

# When callers invoke this shim by absolute path without exporting
# CLAUDE_PLUGIN_ROOT (direct /design fences), derive the plugin root as the
# parent of this script's resolved scripts/ directory. Explicit CLAUDE_PLUGIN_ROOT
# still wins.
resolve_plugin_root_from_script() {
    local script_path=""
    local scripts_dir=""
    local candidate=""
    script_path="${BASH_SOURCE[0]:-$0}"
    scripts_dir="$(CDPATH='' cd -- "$(dirname -- "$script_path")" && pwd -P)" || die "unable to resolve scripts directory from bootstrap script path"
    candidate="$(CDPATH='' cd -- "$scripts_dir/.." && pwd -P)" || die "unable to derive CLAUDE_PLUGIN_ROOT from script location"
    printf '%s\n' "$candidate"
}

main() {
    local version=""
    local target=""
    local override="${LARCH_BINARY:-}"
    local no_install="${LARCH_BOOTSTRAP_NO_INSTALL:-}"

    if [ "${1:-}" = "--preflight-release" ]; then
        [ "$#" -eq 2 ] || die "--preflight-release requires exactly one version"
        preflight_release "$2"
        return 0
    fi
    if [ "${1:-}" = "--latest-stable-version" ]; then
        [ "$#" -eq 1 ] || die "--latest-stable-version accepts no arguments"
        latest_stable_version
        return 0
    fi

    plugin_root="${CLAUDE_PLUGIN_ROOT:-}"
    if [ -z "$plugin_root" ]; then
        plugin_root="$(resolve_plugin_root_from_script)"
    fi
    while [ "$plugin_root" != "/" ] && [ "${plugin_root%/}" != "$plugin_root" ]; do
        plugin_root="${plugin_root%/}"
    done
    validate_absolute_path "CLAUDE_PLUGIN_ROOT" "$plugin_root"
    [ -d "$plugin_root" ] && [ ! -L "$plugin_root" ] || die "CLAUDE_PLUGIN_ROOT is not a real directory"
    # Export the validated root so the exec'd binary and children see the same
    # value when the shim derived it (or stripped trailing slashes).
    export CLAUDE_PLUGIN_ROOT="$plugin_root"
    version="$(read_plugin_version "$plugin_root/.claude-plugin/plugin.json")"
    target="$(resolve_target)"

    if [ -n "$override" ]; then
        if [ "$no_install" = "1" ]; then
            (validate_absolute_path "LARCH_BINARY" "$override") 2>/dev/null \
                || exit "$LARCH_NO_INSTALL_EXIT"
        else
            validate_absolute_path "LARCH_BINARY" "$override"
        fi
        if binary_matches_version "$override" "$version" && binary_passes_self_check "$override" "$version" "$target"; then
            run_binary "$override" "$@"
        fi
        [ "$no_install" = "1" ] && exit "$LARCH_NO_INSTALL_EXIT"
        binary_matches_version "$override" "$version" || die "LARCH_BINARY is not an executable for plugin version $version"
        die "LARCH_BINARY self-check does not match $version for $target"
    fi

    bin_dir="$plugin_root/bin"
    binary_path="$bin_dir/larch"
    if [ -e "$binary_path" ] || [ -L "$binary_path" ]; then
        if [ -f "$binary_path" ] && [ ! -L "$binary_path" ] \
            && binary_matches_version "$binary_path" "$version" \
            && binary_passes_self_check "$binary_path" "$version" "$target"; then
            run_binary "$binary_path" "$@"
        fi
        [ "$no_install" = "1" ] && exit "$LARCH_NO_INSTALL_EXIT"
        [ -f "$binary_path" ] && [ ! -L "$binary_path" ] || die "existing larch binary is not a regular file"
    fi

    [ "$no_install" = "1" ] && exit "$LARCH_NO_INSTALL_EXIT"

    if [ -e "$plugin_root/.git" ] || [ -L "$plugin_root/.git" ]; then
        die "local --plugin-dir checkout needs an explicit build: run 'cargo build --locked --release --package larch-cli' and set LARCH_BINARY to target/release/larch"
    fi

    plugin_data="${CLAUDE_PLUGIN_DATA:-}"
    [ -n "$plugin_data" ] || die "CLAUDE_PLUGIN_DATA is required for the bounded bootstrap lock"
    while [ "$plugin_data" != "/" ] && [ "${plugin_data%/}" != "$plugin_data" ]; do
        plugin_data="${plugin_data%/}"
    done
    validate_absolute_path "CLAUDE_PLUGIN_DATA" "$plugin_data"
    require_commands
    acquire_lock

    if [ -e "$binary_path" ] || [ -L "$binary_path" ]; then
        [ -f "$binary_path" ] && [ ! -L "$binary_path" ] || die "existing larch binary is not a regular file"
        if binary_matches_version "$binary_path" "$version" && binary_passes_self_check "$binary_path" "$version" "$target"; then
            run_binary "$binary_path" "$@"
        fi
    fi

    mkdir -p -- "$bin_dir"
    [ -d "$bin_dir" ] && [ ! -L "$bin_dir" ] || die "plugin bin path is not a real directory"
    chmod 700 "$bin_dir"
    stage_parent="$bin_dir"
    stage_dir="$(mktemp -d "$stage_parent/.larch-bootstrap.XXXXXX")"
    case "$stage_dir" in
        "$stage_parent"/.larch-bootstrap.*) ;;
        *) die "mktemp returned a staging path outside the plugin bin directory" ;;
    esac
    [ -d "$stage_dir" ] && [ ! -L "$stage_dir" ] || die "bootstrap staging path is unsafe"
    install_release_binary "$version" "$target"
    run_binary "$binary_path" "$@"
}

main "$@"
