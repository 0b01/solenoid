SRC := src
OUT := build
LLC := /mnt/c/Users/ricky/Desktop/llvm/build/bin/llc
LLC_ARGS := -filetype=obj -relocation-model=pic -O3
CC := clang
ARGS := -fPIC -O3
EXE := contracts.exe
EXE_SOLANA := contracts.so

LL = $(wildcard $(SRC)/*.ll)
LLO = $(patsubst $(SRC)/%.ll, $(OUT)/%.o, $(LL))
LLO_SOLANA = $(patsubst $(SRC)/%.ll, $(OUT)/%.bpf.o, $(LL))

C_SOURCES = $(wildcard $(SRC)/*.c)
C_OBJECTS = $(patsubst $(SRC)/%.c, $(OUT)/%.o, $(C_SOURCES))
C_OBJECTS_SOLANA = $(patsubst $(SRC)/%.c, $(OUT)/%.bpf.o, $(C_SOURCES))

all: $(OUT) $(EXE)

$(OUT):
	mkdir $(OUT)

$(LLO): $(OUT)/%.o : $(SRC)/%.ll
	$(LLC) $< $(LLC_ARGS) -o $@

$(LLO_SOLANA): $(OUT)/%.bpf.o : $(SRC)/%.ll
	$(LLC) $< $(LLC_ARGS) -march=bpf -o $@

$(C_OBJECTS): $(OUT)/%.o : $(SRC)/%.c
	$(CC) $< $(ARGS) -c -o $@

$(C_OBJECTS_SOLANA): $(OUT)/%.bpf.o : $(SRC)/%.c
	$(CC) $< $(ARGS) -emit-llvm  -c -o $(OUT)/$*.ll
	$(LLC) $(OUT)/$*.ll $(LLC_ARGS) -march=bpf -o $@

$(EXE): $(LLO) $(C_OBJECTS)
	$(CC) $(LLO) $(C_OBJECTS) -o $(OUT)/$(EXE)

$(EXE_SOLANA): $(LLO_SOLANA) $(C_OBJECTS_SOLANA)
	$(CC) $(LLO_SOLANA) $(C_OBJECTS_SOLANA) -o $(OUT)/$(EXE_SOLANA)

clean:
	rm -R $(OUT)

run: all
	./$(OUT)/$(EXE)

solana: $(EXE_SOLANA)