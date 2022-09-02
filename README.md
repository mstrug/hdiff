# Hdiff

Hash diff based delta file updater.

# Usage

## Create signature of a file
```
hdiff signature <input file> <output signature file> [optional chunk size]
```

## Create delta file
```
hdiff delta <signature file> <new input file> <output delta file> [optional chunk size]
```

Default chunk size is 1024 bytes, use values larger than 32 bytes.
