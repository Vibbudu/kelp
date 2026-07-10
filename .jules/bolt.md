## 2026-07-10 - Pre-join search terms to reduce allocations
**Learning:** In highly iterated loops like the file matching process (`search_and_match`), dynamically constructing strings (e.g. `terms.join(" ")`) for every evaluated item causes significant allocation overhead.
**Action:** Compute strings that are constant across the iteration once beforehand (e.g. during query parsing) and cache them in a struct to pass by reference instead.
