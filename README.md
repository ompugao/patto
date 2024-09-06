# Misc
## sort tasks with grep and sort
```sh
grep -r -E '.*@task.*todo' . | awk -F 'until=' '{if (NF>1) print $2, $0; else print "9999/99/99", $0}' | sort | cut -d' ' -f2-
# or, in vim
cgetexpr system("rg --vimgrep '.*@task.*todo' . | awk -F 'until=' '{if (NF>1) print $2, $0; else print \"9999/99/99\", $0}' | sort | cut -d' ' -f2-") | copen

```
