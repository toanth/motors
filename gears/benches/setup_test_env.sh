#!/bin/bash

SCRIPTPATH="$( cd "$(dirname "$0")" ; pwd -P )"

usage() {
    cat << EOF >&2
Usage: $0 [options]
Changes system-wide settings to reduce the variance of benchmarks, requires root privileges.
The changes performed by this script are based on the recommendations from LLVM: https://llvm.org/docs/Benchmarking.html
Options include:

--set:              Applies the changes to reduce benchmark variance. Be careful to read what this does before executing this as
                    forgetting to unset them will leave your system in a state you will probably find undesirable
                    (such as loosing access to four logical CPUs). Also, the previous settings aren't preserved, so in the (unlikely) case
                    that you are using custom settings (such as a cpupower frequency scheduler other than schedutil, these settings will
                    be overwritten)
--unset:            Reverts the system-wide changes to reasonable defaults, which were probably used before this script was executed.
--help:             Print this message.
EOF
}


if [[ $1 == "--help" ]]; then
    usage
    exit 0
fi


# getopt usage inspired by
# https://stackoverflow.com/questions/192249/how-do-i-parse-command-line-arguments-in-bash/29754866#29754866
getopt --test
if [[ $? -ne 4 ]]; then
    echo "getopt didn't work"
fi

LONGOPTS=set,unset
OPTIONS=s,u

PARSED="$(getopt --options=$OPTIONS --longoptions=$LONGOPTS --name "$0" -- "$@")"
if [[ $? -ne 0 ]]; then
    echo
    usage
    exit 2
fi

eval set -- "$PARSED"

moreThan4Cores=0 # if there are more than 4 logical CPUs, reserve 2 for benchmarking and turn off their SMT siblings
if [[ $(nproc) -gt 4 ]]; then
    moreThan4Cores=1 # set that now because cset shield reduces the number of available cores
fi

doUnset=0

while true; do
    case "$1" in
        --set|-s)
            shift
            ;;
        --unset|-u)
            doUnset=1
            shift
            ;;
        --)
            shift
            break
            ;;
        *)
            echo "Unrecognized option"
            usage
            exit 3
            ;;
    esac
done



if [[ $doUnset == 0 ]]; then
    oldGovernor=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)
    echo "cpufreq performance governor was ${oldGovernor}, now set to 'performance'"
    sudo cpupower frequency-set -g performance
    if [[ $moreThan4Cores == 1 ]]; then
        # reserve 2 CPUs so that we can also run perf, which should run on a different CPU
        # `cset shield -c 0-3 -k on` reserves the logical cpus 0 and 1 and moves all threads, including kernel threads, out of them
        sudo cset shield -c 0-3 -k on
        user=$(whoami)
        echo "allowing user ${user} to run in the 'user' cpuset with 'cset exec' without root privileges"
        sudo chown -R ${user} /cpusets/user # the benchmark shouldn't be executed with root privileges
        # typically, cpus 0 and 1 form an SMT pair, as well as cpus 2 and 3
        echo "disabling CPUs 1 and 3, current status: $(sudo cat /sys/devices/system/cpu/cpu1/online) $(sudo cat /sys/devices/system/cpu/cpu3/online)"
        echo 0 | sudo tee /sys/devices/system/cpu/cpu1/online
        echo 0 | sudo tee /sys/devices/system/cpu/cpu3/online
        # don't disable ASLR globally as that would be a security risk. Instead, the benchmark should be run with
        # `setarch $*uname -m) -R` to disable ASLR
    else
        echo "There are 4 CPUs or less on your system, so no CPUs have been reserved for benchmarking"
    fi
else
    if [[ $moreThan4Cores == 1 ]]; then
        echo "Enabling cpus 1 and 3"
        echo 1 | sudo tee /sys/devices/system/cpu/cpu1/online
        echo 1 | sudo tee /sys/devices/system/cpu/cpu3/online
        echo "resetting cset shield"
        sudo cset shield --reset
    fi
    echo "Setting performance governor to 'schedutil'"
    sudo cpupower frequency-set -g schedutil
fi

