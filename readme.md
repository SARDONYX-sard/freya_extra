# freya_extra

Freya is highly flexible, but its API has undergone significant changes. Version 0.3 used the `rsx!` syntax, while version 0.4 introduced a major shift toward a gpui-style API. Even in version 0.5, some argument signatures changed slightly. I decided to test whether I could implement a custom component equivalent to `mui-data-grid`.

It is possible, but Freya lacks several built-in components, so a considerable amount of code is required.

I also found that Freya follows React-style coding patterns, such as the Provider pattern. This makes it easy to introduce mistakes that cause panics, for example, when accessing a context outside its Provider. Even Freya's author has encountered a panic caused by a context issue in the ColorPicker's Menu.

Furthermore, Freya does not fully leverage Rust's strength in preventing mistakes through its type system, making it somewhat less convenient to work with than egui.

Nevertheless, Freya has considerable potential for building clean, well-designed UIs.

![freya_dnd_table](https://github.com/user-attachments/assets/da956bd6-c8b6-4b11-add9-6fe740e68a10)
