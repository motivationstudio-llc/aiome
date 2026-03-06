import os
from pathlib import Path

def generate_wiki():
    base_dir = Path(".")
    docs_dir = base_dir / "docs"
    docs_dir.mkdir(exist_ok=True)

    # Core Wiki Index
    wiki_index = "# 🌌 Documentation Hub - Aiome\n\n"
    wiki_index += "Welcome to the Aiome project documentation. This wiki is automatically generated.\n\n"
    wiki_index += "## 🏗️ Architecture Overviews\n\n"

    # Scan apps and libs
    for category in ["apps", "libs"]:
        wiki_index += f"### {category.capitalize()}\n"
        cat_path = base_dir / category
        if cat_path.exists():
            for item in cat_path.iterdir():
                if item.is_dir():
                    name = item.name
                    # Create a specific doc for each crate
                    crate_doc_path = docs_dir / f"{name}.md"
                    crate_doc_content = f"# 📦 {name}\n\n"
                    crate_doc_content += f"**Category**: {category}\n\n"
                    crate_doc_content += "## 📝 Description\n"
                    crate_doc_content += f"Detailed documentation for the `{name}` crate.\n\n"
                    
                    # Try to find src files
                    src_path = item / "src"
                    if src_path.exists():
                        crate_doc_content += "### 📂 Source Files\n"
                        for src_file in src_path.glob("**/*.rs"):
                            rel_path = src_file.relative_to(item)
                            crate_doc_content += f"- `{rel_path}`\n"
                    
                    with open(crate_doc_path, "w", encoding="utf-8") as f:
                        f.write(crate_doc_content)
                    
                    wiki_index += f"- [{name}](./{name}.md)\n"
        wiki_index += "\n"

    wiki_index += "## 🛡️ Iron Principles\n\n"
    wiki_index += "- **Result Type Mandatory**: `unwrap()` and `expect()` are forbidden.\n"
    wiki_index += "- **Async/Await**: Using `tokio` for non-blocking I/O.\n"
    wiki_index += "- **Workspace Structure**: Strict dependency directions.\n"

    with open(docs_dir / "CLOUD_DOCUMENTATION.md", "w", encoding="utf-8") as f:
        f.write(wiki_index)

    print("Wiki generated successfully in docs/")

if __name__ == "__main__":
    generate_wiki()
