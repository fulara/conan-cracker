#!/bin/bash

#todo:
#1. implement --enable-rh-check
#2. cleanup usage.
#3. ?

function error_exit { echo -e "\e[31m${@}" 1>&2; exit 1; }
function warn { echo -e "\e[33m${@}"; }
function info { echo -e "\e[32m${@}"; }

function usage
{
  cat << EOF
usage: cracker.sh [-r --enable-rh-check] conan_package binaries...
EOF
  if [[ -n "$1" ]]; then
    warn "whereas you invoked:"
    warn "cracker.sh ${@}"
  fi
}

function ask_user {
  while true; do
    read -p "$1 Do you wish to $2 Yn " yn
    case $yn in
      [Yy]* ) return 0;;
      [Nn]* ) return 1;;
      * ) return 0;;
  esac
done
}

function validate_args {
  if [ "${#args[@]}" -lt 2 ]; then
    usage ${orig_args[@]}
    exit 1
  fi
}

function init_cache {
  if [ -z ${CRACKER_CACHE_DIR} ]; then
    CRACKER_CACHE_DIR=~/.cracker_cache
  else
    CRACKER_CACHE_DIR=${CRACKER_CACHE_DIR}/.cracker_cache
  fi

  CRACKER_CACHE_DIR=${CRACKER_CACHE_DIR:-~/.cracker_cache}

  mkdir -p $CRACKER_CACHE_DIR
  mkdir -p $CRACKER_CACHE_DIR/.cracker_storage
}

#positional args
orig_args=( "${@}" )
args=()

# named args
while [ "$1" != "" ]; do
  case "$1" in
    -r | --enable-rh-check )      RH_CHECK=yes;;
    -s | --some_more_args )       some_more_args="$2";     shift;;
    -y | --yet_more_args )        yet_more_args="$2";      shift;;
    -h | --help )                 usage;                   exit;; # quit and show usage
    * )                           args+=("$1")             # if no match, add it to the positional args
    esac
    shift # move to next kv pair
done

# restore positional args
set -- "${args[@]}"

validate_args
init_cache

command -v conan > /dev/null || error_exit "conan is not present in your path"
conan_package=$1

if [[ ! "$conan_package" = *"/"* ]]; then
  error_exit "Selected package:\"$conan_package\" does not contain slash? required form of: name/version."
fi 

if [[ ! "$conan_package" = *"@"* ]]; then
  conan_package=${conan_package}@
fi

if [[ ${#conan_package} -le 5 ]]; then
  error_exit "conan package ${conan_package} provided shorter than 5 characters.. conan does not handle that!"
fi

conan search $conan_package -r=all &> /dev/null  || error_exit "Package $conan_package not found in cache."
echo "conan search $conan_package &> /dev/null "
shift

conan_package_name=$(echo "$conan_package" | cut -d'/' -f 1)
pkg_dir=${CRACKER_CACHE_DIR}/.$conan_package_name
pkg_index=$pkg_dir/.cracker_index

if [[ -d $pkg_dir ]]; then
  if ask_user "Package $conan_package already cracked" "override?"; then
    info "deleting $conan_package from cracker_cache and all its installed packages."
    for p in $(cat $pkg_index 2>/dev/null ); do 
       info "deleting: ${CRACKER_CACHE_DIR}/$p"
       rm -rf ${CRACKER_CACHE_DIR}/$p

    done
    rm -rf $pkg_dir
  else 
    warn "not overriding existing package - exiting."
    exit 0
  fi
fi 

previous_storage_path=$(conan config get storage.path)
conan config set storage.path=${CRACKER_CACHE_DIR}/.cracker_storage
conan install ${conan_package} -g virtualenv -g virtualrunenv -if $pkg_dir 2>&1 || {
  conan config set storage.path=${previous_storage_path}
  echo $output
  error_exit "Failed to install $conan_package_name"
}
conan config set storage.path=${previous_storage_path} || error_exit "Unable to revert your storage path to previous value, your conan installation is now corrupted, previous path was: $previous_storage_path"

while test ${#} -gt 0
do
  what=$1
  location=${CRACKER_CACHE_DIR}/$1
  shift
  if [ -f $location ] ; then
    if ask_user "Location $location is already occupied" "overwrite with $what from $conan_package_name?"; then
      info "overriding $what."
      warn "this will not remove the mention of that package from its creator index! may result in unexpected issues."
      rm $location
    else
      continue
    fi
  fi
  if [ -d $location ]; then
    warn "Location $location is already occupied by a directory - possibly package with $what name exists, skipping"
    continue
  fi
  echo "Installing wrapper for: $what in $location"  
  cat <<EOT >> $location
#!/bin/bash
source $pkg_dir/activate_run.sh
source $pkg_dir/activate.sh
$what "\${@}"
EOT
  chmod +x $location
  echo "$what" >> $pkg_index
done
