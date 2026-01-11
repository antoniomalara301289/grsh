#!/bin/sh
# 1 = Blocco, 3 = Sottolineato, 5 = Barra (Beam)
case $1 in
    1) printf "\033[1 q" ;;
    3) printf "\033[3 q" ;;
    5) printf "\033[5 q" ;;
    *) ;;
esac
