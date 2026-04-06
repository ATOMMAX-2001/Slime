# AUTHOR: S.ABILASH
# Email: abinix01@gmail.com
import os
import shutil
import subprocess as sp
import sys
import time
from pathlib import Path

from watchdog.events import FileSystemEventHandler
from watchdog.observers import Observer


def create_project(name: str):
    if shutil.which("uv") is None:
        print("\n[*] uv is not found installing...")
        command = [sys.executable, "-m", "pip", "install", "uv"]
        sp.run(command, check=True)

    print(f"\n[*] Creating project {name}")
    root = Path.cwd() / name

    static_path = root / "static"
    template_path = root / "templates"

    static_path.mkdir(parents=True, exist_ok=True)
    template_path.mkdir(parents=True, exist_ok=True)

    script_path = root / "main.py"

    code = """
from slimeweb import Slime

app = Slime(__file__)

@app.route(path="/", method="GET")
def home(req, resp):
    return resp.plain("Hello World from slime")

if __name__ == "__main__":
    app.serve(dev=True)
"""

    script_path.write_text(code)

    print("[*] Creating an env\n")
    if not (root / ".venv").exists():
        os.chdir(root)
        try:
            sp.run(
                ["uv", "--native-tls", "venv", "--python", "python3.14t"],
                check=True,
                stdout=sp.DEVNULL,
                stderr=sp.DEVNULL,
            )
            sp.run(
                ["uv", "python", "pin", "python3.14t"],
                check=True,
                stdout=sp.DEVNULL,
                stderr=sp.DEVNULL,
            )
        except Exception as err:
            print("Failed to download the dependency (reason) =>", err)

        sp.run(["uv", "init"])
        sp.run(["uv", "add", "slimeweb"], check=True)
    print(f"\n\n[*] Project '{name}' created 🎉🎉🎉")
    print(f"[*] cd {name} ")

    print("[*] slime run main")


class RestartSlimeHandler(FileSystemEventHandler):
    def __init__(self, path: Path, no_gil: bool) -> None:
        self.script_path: Path = path
        self.process = None
        self.last_run = 0
        self.no_gil = True

    def restart(self):
        now = time.time()
        if now - self.last_run < 1:
            return
        self.last_run = now

        if self.process:
            print("INFO: File Change detected, Restarting...")
            self.process.terminate()
            self.process.wait(timeout=2)
        os.environ["PYTHON_GIL"] = "0" if self.no_gil else "1"
        command = ["uv", "run", "python", str(self.script_path)]
        self.process = sp.Popen(command, env=os.environ.copy())

    def on_modified(self, event):
        if event.is_directory:
            return
        self.restart()


def run_project(script: Path, no_gil: bool = True, auto_reload: bool = False):
    if auto_reload:
        print("INFO: Watching the files", flush=True)
    script_path = Path.cwd()
    if script.suffix == ".py":
        script_path = script_path.joinpath(Path(script))
    else:
        script_path = script_path.joinpath(Path(script).with_suffix(".py"))

    if not script_path.exists():
        print(f"❌ Script '{script_path}' not found")
        sys.exit(1)
    try:
        if auto_reload:
            auto_handler = RestartSlimeHandler(script_path, no_gil)
            observer = Observer()
            observer.schedule(
                auto_handler, path=str(script_path.parent), recursive=True
            )
            auto_handler.restart()
            observer.start()
            try:
                observer.join()
            except KeyboardInterrupt:
                observer.stop()
                observer.join()

        else:
            os.environ["PYTHON_GIL"] = "0" if no_gil else "1"
            command = ["uv", "run", "python", script_path]
            sp.run(command, env=os.environ.copy())
    except KeyboardInterrupt:
        pass
    except Exception as err:
        print("Error Running (reason)=> ", err)


def add_lib(lib: list):
    command = ["uv", "add"]
    command.extend(lib)
    try:
        sp.run(command, check=True)
    except Exception:
        print(
            "Unable to download the package, Verify if it's available for that version or your current Python runtime"
        )


def change_python_version(version):
    try:
        sp.run(["uv", "--native-tls", "run", "python", version], check=True)
    except Exception:
        print(f"Unable to use {version}, No such python runtime available")
        print(
            "These are currently available right now, please choose one of the runtime"
        )
        sp.run(["uv", "--native-tls", "python", "list"])


def display_logo():
    print("   _____ _ _             __          __  _     ")
    print("  / ____| (_)            \\ \\        / / | |    ")
    print(" | (___ | |_ _ __ ___   __\\ \\  /\\  / /__| |__  ")
    print("   ___ \\| | | '_ ` _ \\ / _ \\ \\/  \\/ / _ \\ '_ \\ ")
    print("  ____) | | | | | | | |  __/\\  /\\  /  __/ |_) |")
    print(" |_____/|_|_|_| |_| |_|\\___| \\/  \\/ \\___|_.__/ ")
    print("Version: 0.1.7\t\t\t Author: S.Abilash")


def main():
    display_logo()
    args = sys.argv[1:]

    if not args:
        print("Usage:")
        print("  slime new <project_name>")
        print("  slime run <script>")
        print("  slime rung <script>")
        print("  slime runw <script>")
        print("  slime rungw <script>")
        sys.exit(1)

    command = args[0]

    if command == "new":
        if len(args) != 2:
            print("Usage: slime new <project_name>")
            sys.exit(1)

        create_project(args[1])

    elif command in ["run", "runw"]:
        if len(args) != 2:
            print("Usage: slime run <script>")
            sys.exit(1)

        parent_path = Path(args[0]).parent
        project_path = parent_path.joinpath(Path(args[1]))

        run_project(project_path, auto_reload=command == "runw")
    elif command in ["rung", "rungw"]:
        if len(args) != 2:
            print("Usage: slime rung <script>")
            sys.exit(1)

        parent_path = Path(args[0]).parent
        project_path = parent_path.joinpath(Path(args[1]))
        run_project(project_path, no_gil=False, auto_reload=command == "rungw")
    elif command == "add":
        if len(args) < 2:
            print("Usage: slime add slimeweb")
        add_lib(args[1:])
    elif command == "use":
        if len(args) != 2:
            print("Usage: slime use python3.12")
            sys.exit(1)
        change_python_version(args[1])
    else:
        print(command in ["run", "runw"], flush=True)
        print(f"Unknown command: {command}")


if __name__ == "__main__":
    main()
