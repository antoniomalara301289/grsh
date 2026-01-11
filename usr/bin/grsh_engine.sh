#!/bin/sh
CMD=$1
VAR=$2
VAL=$3

case $CMD in
    "setenv"|"set")
        if [ "$VAR" = "GRSH_CURSOR" ]; then
            [ "$VAL" = "1" ] && printf "\033[1 q"
            [ "$VAL" = "5" ] && printf "\033[5 q"
        fi
        # Qui puoi aggiungere altri comportamenti per altre variabili
        ;;
    "cd")
        # Esempio: aggiorna il titolo del terminale quando cambi cartella
        printf "\033]0; GRSH: $PWD \007"
        ;;
esac
