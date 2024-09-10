# Misc
## sort tasks with grep and sort
```sh
rg --vimgrep '.*@task.*todo' . | awk '{match($0, /until=([0-9:\-T]+)/, m); if (RLENGTH>0) print m[1], $0; else print "9999-99-99", $0}' |sort |cut -d' ' -f2-
# or, in vim
cgetexpr system('rg --vimgrep ".*@task.*todo" . | awk "{match(\$0, /until=([0-9T:\-]+)/, m); if (RLENGTH>0) print m[1], \$0; else print \"9999-99-99\", \$0}" |sort|cut -d" " -f2-')|copen

```
