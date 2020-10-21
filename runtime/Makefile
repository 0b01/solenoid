SRC := src
OUT := bin
LLC := /mnt/c/Users/ricky/Desktop/llvm/build/bin/llc
LLC_ARGS := -filetype=obj -relocation-model=pic -O3
CC := clang
ARGS := -fPIC -O3
EXE := contracts.exe

LL = $(wildcard $(SRC)/*.ll)
LLO = $(patsubst $(SRC)/%.ll, $(OUT)/%.o, $(LL))
C_SOURCES = $(wildcard $(SRC)/*.c)
C_OBJECTS = $(patsubst $(SRC)/%.c, $(OUT)/%.o, $(C_SOURCES))

all: $(OUT) $(EXE)

$(OUT):
	mkdir $(OUT)

$(LLO): $(OUT)/%.o : $(SRC)/%.ll
	$(LLC) $< $(LLC_ARGS) -o $@

$(C_OBJECTS): $(OUT)/%.o : $(SRC)/%.c
	$(CC) $< $(ARGS) -c -o $@

$(EXE): $(LLO) $(C_OBJECTS)
	$(CC) $(LLO) $(C_OBJECTS) -o $(OUT)/$(EXE)

clean:
	rm -R $(OUT)

run: $(EXE) 
	./$(OUT)/$(EXE)
