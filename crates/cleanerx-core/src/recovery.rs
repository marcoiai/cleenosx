pub fn generate_recovery_script() -> String {
    generate_recovery_script_with_targets(&[])
}

pub fn generate_recovery_script_with_targets(targets: &[String]) -> String {
    let queued_targets = if targets.is_empty() {
        "CLEANERX_QUEUED_TARGETS=()".to_string()
    } else {
        format!(
            "CLEANERX_QUEUED_TARGETS=({})",
            targets
                .iter()
                .map(|target| shell_quote(target))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };

    r#"#!/bin/bash
set -u

print() {
  printf '%b\n' "$*"
}

RED=$'\033[31m'
GREEN=$'\033[32m'
YELLOW=$'\033[33m'
ORANGE=$'\033[38;5;208m'
BLUE=$'\033[34m'
GRAY=$'\033[90m'
BOLD=$'\033[1m'
RESET=$'\033[0m'
CLEANERX_PREPARED_SIZE=""
CLEANERX_DATA_PATH=""
CLEANERX_LAST_TEST_TARGET=""
__CLEANERX_QUEUED_TARGETS__

is_root() {
  [[ "$(id -u)" == "0" ]]
}

title() {
  print "\n${BOLD}${BLUE}🧰 cleenosx Recovery Assistant${RESET}"
  print "${GRAY}Guided, explicit cleanup helper for macOS Recovery.${RESET}"
  if is_root; then
    print "${GREEN}✅ Running as root. Protected permission checks and cleanup can proceed.${RESET}"
  else
    print "${YELLOW}⚠️ Not running as root. Scans can work, but cleanup may fail on protected paths.${RESET}"
    print "${GRAY}In normal macOS, run: sudo bash /Users/Shared/cx.sh${RESET}"
    print "${GRAY}In macOS Recovery Terminal, this is usually root already.${RESET}"
  fi
  print "${GRAY}------------------------------------------------------------${RESET}\n"
}

pause() {
  print "\n${GRAY}↩️ Press return to continue...${RESET}"
  read -r _
}

spinner() {
  local pid=$1
  local label=$2
  local start=$(date +%s)
  local frames=('|' '/' '-' '\\')
  local i=0
  while kill -0 "$pid" 2>/dev/null; do
    local now=$(date +%s)
    local elapsed=$(( now - start ))
    printf "\r%s ⏳ %s still running... %ss" "${frames[$i]}" "$label" "$elapsed"
    i=$(( (i + 1) % 4 ))
    sleep 1
  done
  printf "\r✅ %s finished.                         \n" "$label"
}

strong_confirm() {
  local target=$1
  local size=$2
  print "\n${RED}${BOLD}🧨 Destructive action requested${RESET}"
  print "🎯 Target: ${target}"
  print "📏 Estimated recoverable size: ${size}"
  print "⚠️ This cannot be assumed reversible."
  print "Type ${BOLD}DELETE${RESET} to continue:"
  read -r answer
  [[ "$answer" == "DELETE" ]] || return 1
  print "🧷 Type the exact target path to confirm:"
  read -r exact
  [[ "$exact" == "$target" ]]
}

require_root() {
  if is_root; then
    return 0
  fi

  print "${RED}🔒 Cleanup needs root for protected files.${RESET}"
  print "${YELLOW}🔁 Run this same script as root:${RESET}"
  print "  sudo bash \"$0\""
  print "${GRAY}Or boot into macOS Recovery and run the script there.${RESET}"
  return 1
}

is_refused_target() {
  local target=$1
  local data_path=$2
  local assets="${data_path}/System/Library/AssetsV2"

  [[ -z "$target" ]] && return 0
  [[ "$target" != /* ]] && return 0
  [[ "$target" == "/" ]] && return 0
  [[ "$target" == "$data_path" ]] && return 0
  [[ "$target" == "$assets" ]] && return 0
  [[ "$target" == "/System" || "$target" == "/Library" || "$target" == "/Applications" || "$target" == "/Users" ]] && return 0
  [[ "$target" != "$data_path"/* ]] && return 0

  return 1
}

prepare_cleanup_target() {
  local data_path=$1
  local target=$2

  if is_refused_target "$target" "$data_path"; then
    print "${RED}🛡️ Refused unsafe or out-of-scope target: ${target}${RESET}"
    return 1
  fi
  if [[ ! -e "$target" ]]; then
    print "${RED}❌ Target does not exist: ${target}${RESET}"
    return 1
  fi

  print "${BLUE}🧾 Preparing cleanup plan for:${RESET} ${target}"
  local size
  size=$(du -sh "$target" 2>/dev/null | awk '{print $1}')
  [[ -z "$size" ]] && size="unknown"
  CLEANERX_PREPARED_SIZE="$size"
  print "${YELLOW}📌 Prepared target:${RESET} ${target}"
  print "${YELLOW}📏 Estimated recoverable size:${RESET} ${size}"
}

map_target_to_data_volume() {
  local data_path=$1
  local target=$2
  if [[ "$target" == /System/Volumes/Data/* ]]; then
    print "${data_path}/${target#/System/Volumes/Data/}"
  else
    print "$target"
  fi
}

delete_one_target() {
  local data_path=$1
  local target=$2
  prepare_cleanup_target "$data_path" "$target" || return 1
  size="$CLEANERX_PREPARED_SIZE"
  if strong_confirm "$target" "$size"; then
    print "${RED}🗑️ Removing ${target}${RESET}"
    rm -rf -- "$target"
    print "${GREEN}✅ Removed. Returning to enclosing folder:${RESET} $(dirname "$target")"
    du -sh "$(dirname "$target")" 2>/dev/null || true
  else
    print "${GREEN}🛑 Cancelled.${RESET}"
  fi
}

delete_queued_targets() {
  local data_path=$1
  if (( ${#CLEANERX_QUEUED_TARGETS[@]} == 0 )); then
    print "${YELLOW}📭 No queued targets in this script.${RESET}"
    return 1
  fi

  print "\n${BOLD}📋 Queued root/recovery targets${RESET}"
  local index=1
  local target
  for target in "${CLEANERX_QUEUED_TARGETS[@]}"; do
    print "${index}) $(map_target_to_data_volume "$data_path" "$target")"
    index=$(( index + 1 ))
  done
  print "a) Process all queued targets one by one"
  print "return) Back"
  read -r choice

  if [[ "$choice" == "a" || "$choice" == "A" ]]; then
    for target in "${CLEANERX_QUEUED_TARGETS[@]}"; do
      delete_one_target "$data_path" "$(map_target_to_data_volume "$data_path" "$target")"
    done
    return 0
  fi

  if [[ "$choice" =~ ^[0-9]+$ ]]; then
    local selected_index=$((10#$choice - 1))
    if (( selected_index >= 0 && selected_index < ${#CLEANERX_QUEUED_TARGETS[@]} )); then
      delete_one_target "$data_path" "$(map_target_to_data_volume "$data_path" "${CLEANERX_QUEUED_TARGETS[$selected_index]}")"
    fi
  fi
}

list_volumes() {
  local diskutil_output
  local apfs_output
  diskutil_output=$(diskutil list internal 2>&1)
  apfs_output=$(diskutil apfs list 2>&1)

  print "${BOLD}💽 Internal disks and APFS volumes${RESET}"
  interpret_apfs_volumes "$apfs_output"
  interpret_diskutil_partitions "$diskutil_output"
  print "\n${BOLD}🧾 Raw diskutil list internal${RESET}"
  printf '%s\n' "$diskutil_output"
  print "\n${BOLD}🧾 Raw diskutil apfs list${RESET}"
  printf '%s\n' "$apfs_output"
}

interpret_apfs_volumes() {
  local apfs_output=$1

  print "\n${BOLD}📦 APFS volume interpretation${RESET}"
  print "${GRAY}Use the Data volume for cleanup. Leave System, Preboot, Recovery, VM, Update, xART, Hardware, and Baseband alone.${RESET}"
  printf '%-10s %-12s %-28s %-32s %s\n' "ID" "Role" "Name" "Mount" "Meaning"
  printf '%s\n' "$apfs_output" | awk '
    function trim(value) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      return value
    }

    function mount_status(value) {
      if (value == "" || value == "Not Mounted") {
        return "not mounted"
      }
      return value
    }

    function filevault_status(value) {
      if (value == "No") {
        return "FileVault off"
      }
      if (value ~ /Locked/) {
        return "encrypted/locked"
      }
      if (value ~ /Unlocked/) {
        return "encrypted/unlocked"
      }
      if (value != "") {
        return "FileVault " value
      }
      return "FileVault unknown"
    }

    function role_meaning(value, mount_point, filevault) {
      if (value == "Data") {
        if (mount_point == "" || mount_point == "Not Mounted") {
          return "mount this Data volume if it is your macOS install; " filevault_status(filevault)
        }
        return "select this mounted Data volume for cleanup; " filevault_status(filevault)
      }
      if (value == "System") {
        return "sealed macOS system volume; inspect only"
      }
      if (value == "Preboot" || value == "Recovery" || value == "VM" || value == "Update" || value == "xART" || value == "Hardware" || value == "Baseband") {
        return "APFS support volume; leave alone"
      }
      return "non-Data APFS volume; inspect before using"
    }

    function emit_volume() {
      if (id == "") {
        return
      }
      if (name == "") {
        name = "-"
      }
      if (mount_point == "") {
        mount_point = "Not Mounted"
      }
      printf "%-10s %-12s %-28s %-32s %s\n", id, role, name, mount_status(mount_point), role_meaning(role, mount_point, filevault)
      found = 1
    }

    /APFS Volume Disk \(Role\):/ {
      emit_volume()
      line = $0
      sub(/^.*APFS Volume Disk \(Role\):[[:space:]]*/, "", line)
      id = line
      sub(/[[:space:]].*$/, "", id)
      role = line
      if (role ~ /\(/) {
        sub(/^.*\(/, "", role)
        sub(/\).*$/, "", role)
      } else {
        role = "APFS"
      }
      name = ""
      mount_point = ""
      filevault = ""
      next
    }

    id != "" && /^[[:space:]]*Name:/ {
      name = $0
      sub(/^[[:space:]]*Name:[[:space:]]*/, "", name)
      name = trim(name)
      next
    }

    id != "" && /^[[:space:]]*Mount Point:/ {
      mount_point = $0
      sub(/^[[:space:]]*Mount Point:[[:space:]]*/, "", mount_point)
      mount_point = trim(mount_point)
      next
    }

    id != "" && /^[[:space:]]*FileVault:/ {
      filevault = $0
      sub(/^[[:space:]]*FileVault:[[:space:]]*/, "", filevault)
      filevault = trim(filevault)
      next
    }

    END {
      emit_volume()
      if (!found) {
        print "No APFS volume roles parsed. Use the raw diskutil output below."
      }
    }
  '
}

interpret_diskutil_partitions() {
  local diskutil_output=$1

  print "\n${BOLD}🧭 Diskutil partition guide${RESET}"
  print "${GRAY}Partitions and APFS containers are storage wrappers. For cleanup, use a mounted Data volume path, not EFI, Recovery, ISC, or container partitions.${RESET}"
  printf '%-12s %-24s %s\n' "ID" "Kind" "Meaning"
  printf '%s\n' "$diskutil_output" | awk '
    function meaning(line, kind) {
      if (kind == "GUID_partition_scheme") {
        return "partition map; not a cleanup target"
      }
      if (kind == "EFI") {
        return "firmware boot support; leave alone"
      }
      if (kind == "Apple_APFS_ISC") {
        return "Apple Silicon system container; leave alone"
      }
      if (kind == "Apple_APFS_Recovery") {
        return "Recovery support container; leave alone"
      }
      if (kind == "Apple_APFS" && line ~ /Container/) {
        return "physical APFS container store; use APFS volume IDs/paths instead"
      }
      if (kind == "APFS Container") {
        return "synthesized APFS container; inspect volumes inside it"
      }
      if (kind == "APFS Volume" && line ~ /Data/) {
        return "candidate Data volume; select it only if mounted"
      }
      if (kind == "APFS Volume") {
        return "APFS volume; check its role in the APFS summary above"
      }
      if (kind == "APFS Snapshot") {
        return "system snapshot; inspect only"
      }
      return "non-Data partition; inspect before using"
    }

    /^[[:space:]]*[0-9]+:/ {
      line = $0
      id = $NF
      kind = $2
      if (line ~ /APFS Container Scheme/) {
        kind = "APFS Container"
      } else if (line ~ /APFS Snapshot/) {
        kind = "APFS Snapshot"
      } else if (line ~ /APFS Volume/) {
        kind = "APFS Volume"
      }
      printf "%-12s %-24s %s\n", id, kind, meaning(line, kind)
      found = 1
    }

    END {
      if (!found) {
        print "No diskutil partition rows parsed. Use the raw output below."
      }
    }
  '
}

mount_or_unlock_volume() {
  print "\n${BOLD}🔓 Unmounted or locked Data volume${RESET}"
  print "${GRAY}If you do not know the identifier, choose 'List disks and volumes' first and look for the APFS Data volume.${RESET}"
  print "${YELLOW}💽 Enter the Data volume identifier, e.g. disk3s5:${RESET}"
  read -r volume_id
  [[ -z "$volume_id" ]] && print "${RED}❌ No volume selected.${RESET}" && return

  print "${BLUE}🔌 Mounting ${volume_id}.${RESET}"
  if diskutil mount "$volume_id"; then
    return
  fi

  print "${YELLOW}🔐 Mount did not complete. If the volume is encrypted, you may be asked for its password.${RESET}"
  diskutil apfs unlockVolume "$volume_id" || true
  print "${BLUE}🔌 Mounting ${volume_id} after unlock attempt.${RESET}"
  diskutil mount "$volume_id"
}

choose_data_path() {
  local paths=()
  local candidate

  for candidate in /Volumes/*; do
    [[ -d "$candidate" ]] || continue
    if [[ -d "$candidate/System/Library/AssetsV2" || "$candidate" == *" - Data" || "$candidate" == *"Data" ]]; then
      paths+=("$candidate")
    fi
  done

  print "\n${BOLD}📦 Mounted Data volume${RESET}"
  if (( ${#paths[@]} > 0 )); then
    if (( ${#paths[@]} == 1 )); then
      CLEANERX_DATA_PATH="${paths[0]}"
      print "${GREEN}✅ Auto-selected:${RESET} ${CLEANERX_DATA_PATH}"
      return 0
    fi

    local index=1
    for candidate in "${paths[@]}"; do
      print "${index}) ${candidate}"
      index=$(( index + 1 ))
    done
    print "m) ✍️ Type path manually"
    print "${YELLOW}📦 Choose a mounted Data volume:${RESET}"
    read -r choice
    if [[ "$choice" =~ ^[0-9]+$ ]]; then
      local selected_index=$((10#$choice - 1))
      if (( selected_index >= 0 && selected_index < ${#paths[@]} )); then
        CLEANERX_DATA_PATH="${paths[$selected_index]}"
        print "${GREEN}✅ Selected:${RESET} ${CLEANERX_DATA_PATH}"
        return 0
      fi
    fi
    [[ "$choice" == "m" || "$choice" == "M" ]] || {
      print "${RED}❌ No Data volume selected.${RESET}"
      return 1
    }
  else
    print "${YELLOW}⚠️ No mounted Data volume was detected.${RESET}"
    print "${GRAY}Use option 1 to list volumes, then option 2 to mount or unlock only if needed.${RESET}"
    print "m) ✍️ Type path manually"
    print "return) ↩️ Go back"
    read -r choice
    [[ "$choice" == "m" || "$choice" == "M" ]] || return 1
  fi

  print "${YELLOW}📁 Enter the mounted Data path, e.g. /Volumes/Macintosh HD - Data:${RESET}"
  read -r data_path
  if [[ ! -d "$data_path" ]]; then
    print "${RED}❌ Path does not exist: ${data_path}${RESET}"
    return 1
  fi
  CLEANERX_DATA_PATH="$data_path"
  print "${GREEN}✅ Selected:${RESET} ${CLEANERX_DATA_PATH}"
}

guided_start() {
  print "${BOLD}🧭 What this script does${RESET}"
  print "1) 📦 Finds your mounted macOS Data volume."
  print "2) 🧪 Lets you create a harmless test file."
  print "3) 🧷 Deletes only an exact selected target after two confirmations."
  print "${GRAY}💡 Tip: start with the safe test file before touching real cleanup targets.${RESET}"
  choose_data_path || true
}

ensure_data_path() {
  if [[ -n "$CLEANERX_DATA_PATH" && -d "$CLEANERX_DATA_PATH" ]]; then
    return 0
  fi
  choose_data_path
}

create_test_target() {
  local data_path=$1
  local test_dir="${data_path}/Users/Shared/cleenosx-Recovery-Test"
  local target="${test_dir}/delete-me.bin"

  mkdir -p "$test_dir" || {
    print "${RED}❌ Failed to create ${test_dir}.${RESET}"
    return 1
  }

  print "${BLUE}🧪 Creating safe test file:${RESET} ${target}"
  dd if=/dev/zero of="$target" bs=1m count=64 >/dev/null 2>&1 || {
    print "${RED}❌ Failed to create test file.${RESET}"
    return 1
  }

  CLEANERX_LAST_TEST_TARGET="$target"
  print "${GREEN}✅ Created test target:${RESET} ${target}"
  du -sh "$target" 2>/dev/null || true
}

measure_assets() {
  local data_path=$1
  local assets="${data_path}/System/Library/AssetsV2"
  if [[ ! -d "$assets" ]]; then
    print "${YELLOW}🔎 AssetsV2 not found at ${assets}.${RESET}"
    return
  fi

  print "${BOLD}📏 Measuring AssetsV2 known targets${RESET}"
  du -sk "$assets"/com_apple_MobileAsset_* 2>/dev/null | sort -n &
  spinner $! "du AssetsV2"
}

cleanup_menu() {
  local data_path=$1
  local assets="${data_path}/System/Library/AssetsV2"

  while true; do
    print "\n${BOLD}🧹 Cleanup workspace${RESET}"
    print "${GRAY}📦 Data volume:${RESET} ${data_path}"
    if is_root; then
      print "${GREEN}✅ Root mode: enabled${RESET}"
    else
      print "${YELLOW}⚠️ Root mode: off. Delete may fail until you run with sudo or Recovery root.${RESET}"
    fi
    [[ -n "$CLEANERX_LAST_TEST_TARGET" ]] && print "${GRAY}🧪 Last test target:${RESET} ${CLEANERX_LAST_TEST_TARGET}"
    (( ${#CLEANERX_QUEUED_TARGETS[@]} > 0 )) && print "${GRAY}📋 Queued root targets:${RESET} ${#CLEANERX_QUEUED_TARGETS[@]}"
    print "${GREEN}1) 📏 Read-only: measure AssetsV2${RESET}"
    print "${ORANGE}2) 🔎 Review-only: show xrOS/watchOS/tvOS candidates${RESET}"
    print "${ORANGE}3) 🔎 Review-only: show iOS simulator candidates${RESET}"
    print "${GREEN}4) 🧪 Create safe test file in Users/Shared${RESET}"
    print "${RED}5) 📋 Continue queued root cleanup${RESET}"
    print "${RED}6) 🗑️ Delete exact selected file or folder${RESET}"
    print "7) ↩️ Back"
    read -r option

    case "$option" in
      1) measure_assets "$data_path"; pause ;;
      2) find "$assets" -maxdepth 1 \( -name '*xrOS*' -o -name '*watchOS*' -o -name '*appleTVOS*' \) -print 2>/dev/null; pause ;;
      3) find "$assets" -maxdepth 1 -name '*iOSSimulatorRuntime*' -print 2>/dev/null; pause ;;
      4) create_test_target "$data_path"; pause ;;
      5) require_root && delete_queued_targets "$data_path"; pause ;;
      6)
        require_root || { pause; continue; }
        if [[ -n "$CLEANERX_LAST_TEST_TARGET" && -e "$CLEANERX_LAST_TEST_TARGET" ]]; then
          print "${YELLOW}🧪 Use last test target?${RESET} ${CLEANERX_LAST_TEST_TARGET}"
          print "↩️ Press return to use it, or type another exact path:"
          read -r target
          [[ -z "$target" ]] && target="$CLEANERX_LAST_TEST_TARGET"
        else
          print "${YELLOW}🎯 Enter exact path to delete inside:${RESET} ${data_path}"
          read -r target
        fi
        prepare_cleanup_target "$data_path" "$target" || { pause; continue; }
        size="$CLEANERX_PREPARED_SIZE"
        if strong_confirm "$target" "$size"; then
          print "${RED}🗑️ Removing ${target}${RESET}"
          rm -rf -- "$target"
          print "${GREEN}✅ Removed. Returning to enclosing folder:${RESET} $(dirname "$target")"
          du -sh "$(dirname "$target")" 2>/dev/null || true
        else
          print "${GREEN}🛑 Cancelled.${RESET}"
        fi
        pause
        ;;
      7) return ;;
      *) print "${YELLOW}👉 Choose 1-7.${RESET}" ;;
    esac
  done
}

title
guided_start
while true; do
  print "${BOLD}🧰 Menu${RESET}"
  if [[ -n "$CLEANERX_DATA_PATH" ]]; then
    print "${GRAY}📦 Selected Data volume: ${CLEANERX_DATA_PATH}${RESET}"
  else
    print "${GRAY}📭 No Data volume selected yet.${RESET}"
  fi
  print "1) 💽 List disks and volumes"
  print "2) 🔓 Mount or unlock a Data volume if needed"
  print "3) 📦 Select mounted Data volume"
  print "4) 📏 Measure AssetsV2"
  print "5) 🧹 Cleanup workspace"
  print "6) 👋 Exit"
  read -r choice
  case "$choice" in
    1) list_volumes; pause ;;
    2) mount_or_unlock_volume; pause ;;
    3) choose_data_path; pause ;;
    4) ensure_data_path && measure_assets "$CLEANERX_DATA_PATH"; pause ;;
    5) ensure_data_path && cleanup_menu "$CLEANERX_DATA_PATH" ;;
    6) exit 0 ;;
    *) print "${YELLOW}👉 Choose 1-6.${RESET}" ;;
  esac
done
"#
    .replace("__CLEANERX_QUEUED_TARGETS__", &queued_targets)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_script_targets_bash_for_recovery() {
        let script = generate_recovery_script();

        assert!(script.starts_with("#!/bin/bash\n"));
        assert!(script.contains("sudo bash /Users/Shared/cx.sh"));
        assert!(!script.contains("zsh"));
        assert!(!script.contains("<->"));
        assert!(script.contains("APFS volume interpretation"));
        assert!(script.contains("Diskutil partition guide"));
        assert!(script.contains("Mount or unlock a Data volume if needed"));
        assert!(script.contains("CLEANERX_DATA_PATH=\"${paths[0]}\""));
        assert!(script.contains("selected_index=$((10#$choice - 1))"));
        assert!(script.contains("🧰 cleenosx Recovery Assistant"));
        assert!(script.contains("🧹 Cleanup workspace"));
        assert!(script.contains("🗑️ Delete exact selected file or folder"));
    }

    #[test]
    fn recovery_script_quotes_queued_targets_for_bash_arrays() {
        let script = generate_recovery_script_with_targets(&[
            "/System/Volumes/Data/path with spaces".to_string(),
            "/System/Volumes/Data/O'Brien".to_string(),
        ]);

        assert!(script.contains(
            "CLEANERX_QUEUED_TARGETS=('/System/Volumes/Data/path with spaces' '/System/Volumes/Data/O'\\''Brien')"
        ));
    }
}
