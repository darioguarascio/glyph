#!/usr/bin/env python3
"""Compare token consumption across languages for identical programs."""

import os
import sys
from pathlib import Path

PROMPTS = {
    "hello": "Write a complete program that prints \"Hello, World!\" to stdout.",
    "fib": "Write a complete program with a recursive fibonacci function and print fib(10).",
    "fizzbuzz": "Write FizzBuzz: for i from 1 to 100, print Fizz if divisible by 3, Buzz if by 5, FizzBuzz if by both, else the number.",
}

LANGS = ["gl", "py", "js", "c", "rs", "go"]
LANG_NAMES = {
    "gl": "Glyph",
    "py": "Python",
    "js": "JavaScript",
    "c": "C",
    "rs": "Rust",
    "go": "Go",
}


def get_encoder():
    try:
        import tiktoken
        return tiktoken.get_encoding("cl100k_base"), "cl100k_base (GPT-4/4o)"
    except ImportError:
        return None, None


def count_tokens(enc, text: str) -> int:
    if enc:
        return len(enc.encode(text))
    # fallback: rough word-based estimate (~1.3 tokens per word)
    return int(len(text.split()) * 1.3) + len(text) // 8


def load_program(prog_dir: Path, task: str, lang: str) -> str:
    ext = {"gl": "gl", "py": "py", "js": "js", "c": "c", "rs": "rs", "go": "go"}[lang]
    path = prog_dir / f"{task}.{ext}"
    return path.read_text()


def main():
    root = Path(__file__).resolve().parent.parent
    prog_dir = root / "benchmarks" / "programs"
    enc, enc_name = get_encoder()

    if not enc:
        print("note: install tiktoken for accurate counts (pip install tiktoken)\n")

    tasks = ["hello", "fib", "fizzbuzz"]
    results = {t: {} for t in tasks}

    for task in tasks:
        prompt = PROMPTS[task]
        prompt_tokens = count_tokens(enc, prompt)
        for lang in LANGS:
            code = load_program(prog_dir, task, lang)
            code_tokens = count_tokens(enc, code)
            total = prompt_tokens + code_tokens
            results[task][lang] = {
                "prompt": prompt_tokens,
                "code": code_tokens,
                "total": total,
                "chars": len(code),
            }

    # Print comparison tables
    print(f"# Glyph Token Benchmark")
    print(f"Encoder: {enc_name or 'heuristic estimate'}\n")

    for task in tasks:
        print(f"## {task.title()} — \"{PROMPTS[task][:60]}...\"")
        print(f"| Language   | Prompt | Code | Total | Chars | Savings vs C |")
        print(f"|------------|--------|------|-------|-------|--------------|")
        baseline = results[task]["c"]["total"]
        for lang in LANGS:
            r = results[task][lang]
            savings = (1 - r["total"] / baseline) * 100 if baseline else 0
            marker = " **" if lang == "gl" else ""
            name = LANG_NAMES[lang] + marker
            print(f"| {name:<10} | {r['prompt']:>6} | {r['code']:>4} | {r['total']:>5} | {r['chars']:>5} | {savings:>+11.1f}% |")
        print()

    # Summary row
    print("## Aggregate (all 3 programs)")
    print("| Language   | Total Tokens | vs C    |")
    print("|------------|--------------|---------|")
    totals = {lang: sum(results[t][lang]["total"] for t in tasks) for lang in LANGS}
    baseline_sum = totals["c"]
    for lang in LANGS:
        s = (1 - totals[lang] / baseline_sum) * 100
        marker = " ← Glyph" if lang == "gl" else ""
        print(f"| {LANG_NAMES[lang]:<10} | {totals[lang]:>12} | {s:>+6.1f}%{marker} |")

    # Write markdown report
    report_path = root / "benchmarks" / "TOKEN_COMPARISON.md"
    with open(report_path, "w") as f:
        f.write("# Token Comparison: Glyph vs Traditional Languages\n\n")
        f.write(f"Measured with **{enc_name or 'heuristic estimate'}**.\n\n")
        f.write("Each row = same natural-language prompt + generated program source.\n\n")
        for task in tasks:
            f.write(f"### {task.title()}\n\n")
            f.write(f"**Prompt:** {PROMPTS[task]}\n\n")
            f.write("| Language | Prompt | Code | **Total** | Chars |\n")
            f.write("|----------|--------|------|-----------|-------|\n")
            for lang in LANGS:
                r = results[task][lang]
                bold = "**" if lang == "gl" else ""
                f.write(f"| {LANG_NAMES[lang]} | {r['prompt']} | {r['code']} | {bold}{r['total']}{bold} | {r['chars']} |\n")
            f.write("\n")
        f.write("### Aggregate\n\n")
        f.write("| Language | Total Tokens | Savings vs C |\n")
        f.write("|----------|--------------|-------------|\n")
        for lang in LANGS:
            s = (1 - totals[lang] / baseline_sum) * 100
            f.write(f"| {LANG_NAMES[lang]} | {totals[lang]} | {s:+.1f}% |\n")

    print(f"\nreport written to {report_path}")


if __name__ == "__main__":
    main()
