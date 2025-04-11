module [
    run!,
]

import Host

run! : {} => Result {} Str
run! = Host.impact_run!
