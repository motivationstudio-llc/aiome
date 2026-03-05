#!/bin/bash
# scripts/apply_license.sh
# Applies AGPL-3.0 license header to all .rs files in the workspace.

LICENSE_HEADER="/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */"

find . -name "*.rs" -not -path "*/target/*" | while read -r file; do
    if ! grep -q "GNU Affero General Public License" "$file"; then
        echo "Applying license to $file"
        echo -e "$LICENSE_HEADER\n\n$(cat "$file")" > "$file"
    fi
done
