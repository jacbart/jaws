#!/usr/bin/env bash

set -e

TAG=$(gh release list | awk '{print $1; exit}')
VERSION=${TAG:1}
TAR_FILE="jaws_${VERSION}_$(uname -s)_$(uname -m).tar.gz"
DLDIR="${TMPDIR}jaws_install"

function check_deps() {
  echo "checking deps..."
  quit=false

  if ! command -v tar &> /dev/null; then
    echo "missing tar, install instructions here https://command-not-found.com/tar"
    quit=true
  fi

  if ! command -v gh &> /dev/null; then
    echo "missing github cli tool, install instruction here https://github.com/cli/cli#installation"
    quit=true
  elif ! gh auth status &> /dev/null; then
    echo "github cli is not authenticated with github"
    echo "either set the env var 'GH_TOKEN' or run 'gh auth login'"
    quit=true
  fi

  if $quit; then
    exit 1
  fi
}

function download_release() {
  echo "downloading ${TAR_FILE}"
  gh release download --repo github.com/jacbart/jaws "${TAG}" --pattern "${TAR_FILE}"
  mkdir -p "${DLDIR}"
  echo "extracting binary to ${DLDIR}"
  tar -xf "${TAR_FILE}" -C "${DLDIR}"
  rm $TAR_FILE
}

function install_release() {
  IFS=':'
  read -a patharr <<< "$PATH"
  LOCS=()
  for index in "${patharr[@]}";
  do
    if [[ "$index" == "$HOME"* ]] || [[ "$index" == *"local"* ]];
    then
      LOCS+=("${index}")
    fi
  done

  PS3='Select jaws install location, enter 0 to exit: '
  select file in "${LOCS[@]}"; do
    if [[ $REPLY == "0" ]]; then
        echo 'quitting...' >&2
        exit 0
    elif [[ -z $file ]]; then
        echo 'Invalid choice, try again' >&2
    else
        break
    fi
  done

  echo "moving jaws binary to ${file}"
  mv "${DLDIR}/jaws" "${file}/jaws"
}

function setup_jaws() {
  echo "creating ${HOME}/.jaws"
  mkdir -p "${HOME}/.jaws"
  chmod -R 0740 $HOME/.jaws

  if [ -f "${HOME}/.jaws/jaws.conf" ]; then
    echo "jaws.conf detected"
  else
    echo "creating jaws.conf"
    jaws config create > "${HOME}/.jaws/jaws.conf"
  fi
  chmod 0640 $HOME/.jaws/jaws.conf
}

check_deps
download_release
install_release
setup_jaws
