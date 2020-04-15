#!/bin/bash

#todo:
#* deduce binaries
#*. cleanup usage.
#*. ?

function error_exit { echo -e "\e[31m${@}" 1>&2; exit 1; }
function warn { echo -e "\e[33m${@}"; }
function info { echo -e "\e[32m${@}"; }

function usage
{
  cat << EOF
usage: cracker.sh [-r --disable-rh-check] [-d --deduce ] [-g --global-cache] conan_package [binaries...]
EOF
  if [[ -n "$1" ]]; then
    warn "whereas you invoked:"
    warn "cracker.sh ${@}"
  fi
}

function rh6_check {
  if [ -n $skip_rh_check ]; then
    return 0
  fi
  if [ -f /etc/redhat-release ]; then
    major_v=$(sed -r  's@.*release ([0-9])+\..*@\1@g' /etc/redhat-release)
    if [[ "$major_v" != 6 ]]; then
      error_exit "you are not on rh6! if you want to have a single .cracker_storage consider using this on rh6."
    fi
  else
    error_exit "you are not on rh/centos. disable this option!"
  fi
}

function ask_user {
  while true; do
    read -p "$1 Do you wish to $2 Yn " yn
    case $yn in
      [Yy]* ) return 0;;
      [Nn]* ) return 1;;
      * ) echo "yn please.";
  esac
done
}

function validate_args {
  expected_arg_count=2
  if [[ -n "$deduce_binaries" ]]; then
    expected_arg_count=1
  fi
  if [ "${#args[@]}" -lt $expected_arg_count ]; then
    usage ${orig_args[@]}
    exit 1
  fi
  rh6_check
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

function crack {
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
}

#positional args
orig_args=( "${@}" )
args=()

# named args
while [ "$1" != "" ]; do
  case "$1" in
    -r | --disable-rh-check )     skip_rh_check=yes;;
    -d | --deduce ) deduce_binaries=yes;;
    -g | --global-storage ) global_storage=yes;;
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
#if global storage is set - we dont override storage.path.
if [[ -z $global_storage ]]; then
  conan config set storage.path=${CRACKER_CACHE_DIR}/.cracker_storage
fi
conan install ${conan_package} -g virtualenv -g virtualrunenv -if $pkg_dir 2>&1 || {
  conan config set storage.path=${previous_storage_path}
  echo $output
  error_exit "Failed to install $conan_package_name"
}
conan config set storage.path=${previous_storage_path} || error_exit "Unable to revert your storage path to previous value, your conan installation is now corrupted, previous path was: $previous_storage_path"

if [[ -n $deduce_binaries ]]; then
  pkg_path=$(sed -n -r 's@^PATH="([^"]+).*@\1@p' $pkg_dir/environment_run.sh.env)
  if [[ -z $pkg_path ]] || [[ ! -d $pkg_path ]]; then
    error_exit "You wanted binary deduction but your binary does not seem to have valid PATH?"
  fi
  for bin in $(find $pkg_path -maxdepth 1 -type f -executable -printf "%f\n" ); do
    crack $bin
  done
fi

while test ${#} -gt 0; do
  crack $1
done
