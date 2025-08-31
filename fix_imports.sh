#!/bin/bash

# Fix standard library imports
files=$(find . -name "*.zen" -type f | xargs grep -l 'build\.import(')

for file in $files; do
    echo "Fixing imports in: $file"
    
    # Replace build.import() with @std. for standard modules
    sed -i 's/build\.import("io")/@std.io/g' "$file"
    sed -i 's/build\.import("fs")/@std.fs/g' "$file"
    sed -i 's/build\.import("string")/@std.string/g' "$file"
    sed -i 's/build\.import("mem")/@std.mem/g' "$file"
    sed -i 's/build\.import("math")/@std.math/g' "$file"
    sed -i 's/build\.import("vec")/@std.vec/g' "$file"
    sed -i 's/build\.import("hashmap")/@std.hashmap/g' "$file"
    sed -i 's/build\.import("collections")/@std.collections/g' "$file"
    sed -i 's/build\.import("json")/@std.json/g' "$file"
    sed -i 's/build\.import("network")/@std.network/g' "$file"
    sed -i 's/build\.import("http")/@std.http/g' "$file"
    sed -i 's/build\.import("process")/@std.process/g' "$file"
    sed -i 's/build\.import("regex")/@std.regex/g' "$file"
    sed -i 's/build\.import("net")/@std.net/g' "$file"
    
    # Remove build := @std.build lines if they're no longer needed
    # Only if build is not used elsewhere in the file
    if ! grep -q 'build\.' "$file" | grep -v 'build := @std.build'; then
        sed -i '/^build := @std\.build$/d' "$file"
    fi
done

echo "Import fixes complete!"