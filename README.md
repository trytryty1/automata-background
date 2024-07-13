# Automata Desktop Background

Automata Desktop Background is an application designed to replace the Windows desktop with a cellular automata simulation.

## Usage

To run the application, use the following command:

```bash
cargo run
```

## How I Made It
- I used the cargo and winit crate to create a window.
- I referred to this [Link Text]([URL](https://www.codeproject.com/Articles/856020/Draw-Behind-Desktop-Icons-in-Windows-plus)) article to figure out how to put it on the desktop, under the shortcuts. article to figure out how to put it on the desktop, under the shortcuts.
- I then used the wgpu library to run hardware-accelerated graphics.

## TODO
- Refresh the desktop background to eliminate artifacts after closing.
- Simulate the automata using a compute shader instead of on the CPU.
- Add more controls to the tray icon.

## Demo
[![IMAGE ALT TEXT](http://img.youtube.com/vi/guEKLNM5alU/0.jpg)](http://www.youtube.com/watch?v=guEKLNM5alU "Automata Desktop Background")
