# Examples of Search Queries

Here are some examples showcasing how you can use the search syntax in Urocissa:

### 1. Search by Any

```
any: "samsung"
```

Search for data that matches the **samsung** in any field.

### 2. Simple Extension Search

```
ext: "jpeg"
```

Search for data extension that is **jpeg**.

### 3. Search by Model and Type

```
or(model: "SLT-A57", type: "image", model: "SLT-A58")
```

Search for data related to either camera model SLT-A57 or SLT-A58, or image type.

### 4. Search by Tag

```
tag: "nature"
```

Search for data that is tagged with **nature**.

### 5. Search by Make

```
make: "nikon"
```

Search for data related to the **Nikon** make.

### 6. Not Expression Search

```
not(model: "outdated")
```

Search for data that does **not** have the **outdated** model.

### 7. Complex Combination

```
and(type: "image", not(tag: "private"), or(any: "sony", any: "samsung"))
```

Search for **image type** data, excluding items tagged as **private**, and including items that have **sony** or **samsung** in any field.
