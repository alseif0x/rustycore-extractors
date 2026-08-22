R"(mmaps_generator options

--threads <count>              Maximum worker threads (default: 3)
--offMeshInput <file>          Off-mesh connection input file
--silent                       Disable interactive waits
--bigBaseUnit <true|false>     Use the larger base unit
--maxAngle <45..90>            Maximum walkable angle (default: 60)
--maxAngleNotSteep <45..90>    Non-steep maximum angle
--skipLiquid <true|false>      Exclude liquid geometry
--skipContinents <true|false>  Skip continent maps
--skipJunkMaps <true|false>    Skip transport/development maps
--skipBattlegrounds <true|false> Skip battleground and arena maps
--debugOutput <true|false>     Write Recast debug meshes
--tile <x,y>                   Build one tile (requires a map id)
--file <path>                  Build one intermediate mesh file
--help                         Show this message

Pass a map id as a positional argument to build only that map.
)"
