#!/bin/bash

function ask_user {
  while true; do
    read -p "$1 Do you wish to $2 Yn " yn
    case $yn in
      [Yy]* ) return 0; break;;
      [Nn]* ) return 1;;
      * ) return 0;;
  esac
done
}

if [ -z ${CRACKER_CACHE_DIR} ]; then
  CRACKER_CACHE_DIR=~/.cracker_cache
else
  CRACKER_CACHE_DIR=${CRACKER_CACHE_DIR}/.cracker_cache
fi

CRACKER_CACHE_DIR=${CRACKER_CACHE_DIR:-~/.cracker_cache}

mkdir -p $CRACKER_CACHE_DIR

if [ "$#" -lt 2 ]; then
  echo "This script requires at least 2 args: [conan-package] [binaries...]"
  exit 1
fi

if ! command -v conan > /dev/null ; then
  echo "conan is not present in your path."
  exit 1
fi

conan_package=$1

if [[ ! "$1" = *"@"* ]]; then
  conan_package=${conan_package}@
fi

conan search $conan_package &> /dev/null  || ( echo "Package $conan_package not found in cache." ; exit 2 )
shift

conan_package_name=$(echo "$conan_package" | cut -d'/' -f 1)
pkg_dir=${CRACKER_CACHE_DIR}/$conan_package_name
pkg_index=$pkg_dir/.cracker_index

if [[ -d $pkg_dir ]]; then
  if ask_user "Package $conan_package already cracked" "override?"; then
    echo "deleting $conan_package from cracker_cache and all its installed packages."
    for p in $(cat $pkg_index 2>/dev/null ); do 
       echo "deleting: ${CRACKER_CACHE_DIR}/$p"
       rm -rf ${CRACKER_CACHE_DIR}/$p

    done
    rm -rf $pkg_dir
  else 
    echo "not overriding existing package - exiting."
    exit 0
  fi
fi 

conan install $conan_package -g virtualenv -g virtualrunenv -if $pkg_dir &>/dev/null || ( echo "Failed to install $conan_package_name" && exit 3 )

while test ${#} -gt 0
do
  what=$1
  location=${CRACKER_CACHE_DIR}/$1
  shift
  if [ -f $location ] ; then
    if ask_user "Location $location is already occupied" "overwrite with $what from $conan_package_name?"; then
      echo "overriding $what."
      echo "WARN: this will not remove the mention of that package from its creator index! may result in unexpected issues." 
      rm $location
    else
      continue
    fi
  fi
  if [ -d $location ]; then
    echo "Location $location is already occupied by a directory - possibly package with $what name exists, skipping"
    continue
  fi
  echo "Installing wrapper for: $what in $location"  
  cat <<EOT >> $location
#!/bin/bash
source $pkg_dir/activate_run.sh
source $pkg_dir/activate.sh
$what "\${@}"
EOT
  echo "$what" >> $pkg_index
done
  chmod +x $location
