#!/usr/bin/env bash

blue=$(tput -T xterm setaf 4)
light_blue=$(tput -T xterm setaf 6)
green=$(tput -T xterm setaf 2)
reset=$(tput -T xterm sgr0)
yellow=$(tput -T xterm setaf 3)

function label() {
  echo
  echo "${green}$1${reset}"
}

function light_blue() {
  echo "${light_blue}$*${reset}"
}

function run() {
  echo "${blue}Running ${yellow}$*${reset}"
  eval "$@"
}
