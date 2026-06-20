pub fn generate_recovery_script() -> String {
    r#"#!/bin/zsh
set -u

RED=$'\033[31m'
GREEN=$'\033[32m'
YELLOW=$'\033[33m'
ORANGE=$'\033[38;5;208m'
BLUE=$'\033[34m'
GRAY=$'\033[90m'
BOLD=$'\033[1m'
RESET=$'\033[0m'

title() {
  print "\n${BOLD}${BLUE}CleanerX Recovery Assistant${RESET}"
  print "${GRAY}Safe, guided, explicit cleanup helper for macOS Recovery.${RESET}"
  print "${GRAY}------------------------------------------------------------${RESET}\n"
}

pause() {
  print "\n${GRAY}Press return to continue...${RESET}"
  read -r _
}

spinner() {
  local pid=$1
  local label=$2
  local start=$(date +%s)
  local frames=('|' '/' '-' '\\')
  local i=1
  while kill -0 "$pid" 2>/dev/null; do
    local now=$(date +%s)
    local elapsed=$(( now - start ))
    printf "\r%s %s still running... %ss" "${frames[$i]}" "$label" "$elapsed"
    i=$(( (i % 4) + 1 ))
    sleep 1
  done
  printf "\r✓ %s finished.                         \n" "$label"
}

strong_confirm() {
  local target=$1
  print "\n${RED}${BOLD}Destructive action requested${RESET}"
  print "Target: ${target}"
  print "This cannot be assumed reversible."
  print "Type ${BOLD}DELETE${RESET} to continue, anything else cancels:"
  read -r answer
  [[ "$answer" == "DELETE" ]]
}

list_volumes() {
  print "${BOLD}Internal disks and APFS volumes${RESET}"
  diskutil list internal
  print ""
  diskutil apfs list
}

unlock_volume() {
  print "${YELLOW}Enter the Data volume identifier, e.g. disk3s5:${RESET}"
  read -r volume_id
  [[ -z "$volume_id" ]] && print "${RED}No volume selected.${RESET}" && return
  print "${BLUE}Unlocking ${volume_id}. You may be asked for the FileVault password.${RESET}"
  diskutil apfs unlockVolume "$volume_id"
  print "${BLUE}Mounting ${volume_id}.${RESET}"
  diskutil mount "$volume_id"
}

choose_data_path() {
  print "${YELLOW}Enter the mounted Data path, e.g. /Volumes/HD - Data:${RESET}"
  read -r data_path
  if [[ ! -d "$data_path" ]]; then
    print "${RED}Path does not exist: ${data_path}${RESET}"
    return 1
  fi
  print "$data_path"
}

measure_assets() {
  local data_path=$1
  local assets="${data_path}/System/Library/AssetsV2"
  if [[ ! -d "$assets" ]]; then
    print "${YELLOW}AssetsV2 not found at ${assets}.${RESET}"
    return
  fi

  print "${BOLD}Measuring AssetsV2 known targets${RESET}"
  du -sk "$assets"/com_apple_MobileAsset_* 2>/dev/null | sort -n &
  spinner $! "du AssetsV2"
}

cleanup_menu() {
  local data_path=$1
  local assets="${data_path}/System/Library/AssetsV2"
  print "\n${BOLD}Cleanup options${RESET}"
  print "${GREEN}1) Read-only: measure again${RESET}"
  print "${ORANGE}2) Review-only: show xrOS/watchOS/tvOS candidates${RESET}"
  print "${ORANGE}3) Review-only: show iOS simulator candidates${RESET}"
  print "${RED}4) Dangerous placeholder: delete selected path manually${RESET}"
  print "5) Cancel"
  read -r option

  case "$option" in
    1) measure_assets "$data_path" ;;
    2) find "$assets" -maxdepth 1 \( -name '*xrOS*' -o -name '*watchOS*' -o -name '*appleTVOS*' \) -print 2>/dev/null ;;
    3) find "$assets" -maxdepth 1 -name '*iOSSimulatorRuntime*' -print 2>/dev/null ;;
    4)
      print "${YELLOW}Enter exact path to delete. Whole AssetsV2 is refused:${RESET}"
      read -r target
      if [[ "$target" == "$assets" || "$target" == "/" || -z "$target" ]]; then
        print "${RED}Refused unsafe target.${RESET}"
        return
      fi
      if strong_confirm "$target"; then
        print "${RED}MVP generated script is still safe-mode: remove command is intentionally commented.${RESET}"
        print "To execute in a future reviewed script: rm -rf -- '$target'"
      else
        print "${GREEN}Cancelled.${RESET}"
      fi
      ;;
    *) print "${GREEN}Cancelled.${RESET}" ;;
  esac
}

title
while true; do
  print "${BOLD}Menu${RESET}"
  print "1) List disks and volumes"
  print "2) Unlock and mount a FileVault Data volume"
  print "3) Measure AssetsV2 on mounted Data volume"
  print "4) Cleanup review menu"
  print "5) Exit"
  read -r choice
  case "$choice" in
    1) list_volumes; pause ;;
    2) unlock_volume; pause ;;
    3) data_path=$(choose_data_path) && measure_assets "$data_path"; pause ;;
    4) data_path=$(choose_data_path) && cleanup_menu "$data_path"; pause ;;
    5) exit 0 ;;
    *) print "${YELLOW}Choose 1-5.${RESET}" ;;
  esac
done
"#
    .to_string()
}
