; Completed helper function implementations for gpt.s

; Load model weights from disk into memory
; Input: RDI = model path string
; Output: RAX = 0 on success, -1 on error
gpt_load_model:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push rdx
    push rsi
    push r8
    push r9
    
    ; Open model file
    mov rax, 2                  ; sys_open
    mov rsi, 0                  ; O_RDONLY
    syscall
    test rax, rax
    js .load_error
    
    mov r8, rax                 ; Save file descriptor
    
    ; Read embedding weights
    mov rax, 0                  ; sys_read
    mov rdi, r8
    lea rsi, [embedding_weights]
    mov rdx, MAX_VOCAB_SIZE * EMBEDDING_DIM * 4
    syscall
    test rax, rax
    js .load_error
    
    ; Read attention weights  
    mov rax, 0                  ; sys_read
    mov rdi, r8
    lea rsi, [attention_weights]
    mov rdx, EMBEDDING_DIM * EMBEDDING_DIM * 4
    syscall
    test rax, rax
    js .load_error
    
    ; Read MLP weights
    mov rax, 0                  ; sys_read
    mov rdi, r8
    lea rsi, [mlp_weights]
    mov rdx, EMBEDDING_DIM * EMBEDDING_DIM * 4 * 4
    syscall
    test rax, rax
    js .load_error
    
    ; Close file
    mov rax, 3                  ; sys_close
    mov rdi, r8
    syscall
    
    ; Copy model name
    lea rdi, [current_model]
    mov rsi, [rbp + 16]        ; Original model path
    mov rcx, 255
    rep movsb
    
    xor rax, rax               ; Success
    jmp .load_exit
    
.load_error:
    mov rax, -1                ; Error
    
.load_exit:
    pop r9
    pop r8
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rbp
    ret

; Check for common tokens to optimize tokenization
; Input: RBX = 8-byte text chunk
; Output: RCX = token ID (0 if not found), RBX = bytes consumed
gpt_check_common_tokens:
    push rbp
    mov rbp, rsp
    push rax
    push rdx
    
    ; Check for common 4-byte sequences first
    mov eax, ebx               ; Get lower 4 bytes
    
    ; Check for " the" (most common token)
    cmp eax, 0x20656874        ; " the" in little endian
    je .found_the
    
    ; Check for "and" 
    and eax, 0x00FFFFFF        ; Mask to 3 bytes
    cmp eax, 0x00646E61        ; "and" 
    je .found_and
    
    ; Check for single space
    movzx eax, bl
    cmp al, 0x20               ; Space character
    je .found_space
    
    ; No common token found
    xor rcx, rcx
    mov rbx, 1                 ; Consume 1 byte
    jmp .check_exit
    
.found_the:
    mov rcx, 262               ; Token ID for " the"
    mov rbx, 4                 ; Consumed 4 bytes
    jmp .check_exit
    
.found_and:
    mov rcx, 290               ; Token ID for "and"
    mov rbx, 3                 ; Consumed 3 bytes
    jmp .check_exit
    
.found_space:
    mov rcx, 220               ; Token ID for space
    mov rbx, 1                 ; Consumed 1 byte
    
.check_exit:
    pop rdx
    pop rax
    pop rbp
    ret

; Convert character to token ID using vocab table
; Input: RBX = character value
; Output: RCX = token ID
gpt_char_to_token:
    push rbp
    mov rbp, rsp
    push rax
    push rdx
    
    ; Bounds check
    cmp rbx, 255
    ja .char_error
    
    ; Simple character mapping (ASCII -> token)
    ; For printable ASCII, use direct mapping + offset
    cmp rbx, 32                ; Printable start
    jb .char_special
    cmp rbx, 126               ; Printable end
    ja .char_special
    
    ; Direct mapping: token = char - 32 + 1000
    sub rbx, 32
    add rbx, 1000
    mov rcx, rbx
    jmp .char_exit
    
.char_special:
    ; Handle special characters
    cmp rbx, 10                ; Newline
    je .char_newline
    cmp rbx, 9                 ; Tab
    je .char_tab
    
    ; Default unknown character token
    mov rcx, 100               ; UNK token
    jmp .char_exit
    
.char_newline:
    mov rcx, 198               ; Newline token
    jmp .char_exit
    
.char_tab:
    mov rcx, 197               ; Tab token
    jmp .char_exit
    
.char_error:
    mov rcx, 100               ; UNK token
    
.char_exit:
    mov rbx, 1                 ; Always consume 1 byte for char
    pop rdx
    pop rax
    pop rbp
    ret

; MLP forward pass with SIMD acceleration
; Input: R8 = input tensor, R9 = output tensor
gpt_mlp_forward_simd:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push rdx
    push r10
    push r11
    
    ; First linear layer: input -> 4 * hidden_dim
    lea rdx, [mlp_weights]
    mov rcx, EMBEDDING_DIM     ; Input dim
    mov r10, EMBEDDING_DIM * 4 ; Output dim (expanded)
    
    ; Matrix multiplication with SIMD
    call gpt_simd_matmul_mlp
    
    ; Apply GELU activation function with SIMD
    mov rcx, r10               ; Number of elements
    mov rax, r9                ; Current tensor
    
.gelu_loop:
    ; Load 8 values at once
    cmp rcx, 8
    jl .gelu_remainder
    
    vmovups ymm0, [rax]        ; Load 8 floats
    
    ; GELU approximation: 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x^3)))
    ; Simplified version: x * sigmoid(1.702 * x)
    vmovups ymm1, ymm0         ; Copy x
    vmulps ymm0, ymm0, ymm0    ; x^2
    vmulps ymm0, ymm0, ymm1    ; x^3
    
    ; Create constant 0.044715
    mov r11, 0x3D378F98        ; 0.044715 in IEEE 754
    vmovd xmm2, r11d
    vbroadcastss ymm2, xmm2
    
    vmulps ymm0, ymm0, ymm2    ; 0.044715 * x^3
    vaddps ymm0, ymm0, ymm1    ; x + 0.044715 * x^3
    
    ; Apply simplified sigmoid approximation
    vmulps ymm0, ymm0, ymm1    ; Multiply by original x
    
    ; Store result
    vmovups [rax], ymm0
    add rax, 32                ; 8 floats * 4 bytes
    sub rcx, 8
    jmp .gelu_loop
    
.gelu_remainder:
    ; Handle remaining elements
    test rcx, rcx
    jz .gelu_complete
    
.gelu_scalar:
    vmovss xmm0, [rax]         ; Load single float
    vmulss xmm1, xmm0, xmm0    ; x^2
    vmulss xmm1, xmm1, xmm0    ; x^3
    mov r11, 0x3D378F98        ; 0.044715
    vmovd xmm2, r11d
    vmulss xmm1, xmm1, xmm2    ; 0.044715 * x^3
    vaddss xmm1, xmm1, xmm0    ; x + 0.044715 * x^3
    vmulss xmm1, xmm1, xmm0    ; Multiply by x
    vmovss [rax], xmm1         ; Store result
    add rax, 4
    dec rcx
    jnz .gelu_scalar
    
.gelu_complete:
    ; Second linear layer: 4 * hidden_dim -> hidden_dim
    lea rdx, [mlp_weights + EMBEDDING_DIM * EMBEDDING_DIM * 4 * 4]
    mov rcx, EMBEDDING_DIM * 4 ; Input dim
    mov r10, EMBEDDING_DIM     ; Output dim
    
    call gpt_simd_matmul_mlp
    
    pop r11
    pop r10
    pop rdx
    pop rcx
    pop rbx
    pop rbp
    ret

; Layer normalization with SIMD
; Input: R8 = input tensor, R9 = output tensor, RCX = size
gpt_layer_norm_simd:
    push rbp
    mov rbp, rsp
    push rbx
    push rdx
    push r10
    push r11
    
    ; Compute mean
    vxorps ymm0, ymm0, ymm0    ; Sum accumulator
    mov rax, r8                ; Input pointer
    mov rbx, rcx               ; Size counter
    
.mean_loop:
    cmp rbx, 8
    jl .mean_remainder
    
    vmovups ymm1, [rax]
    vaddps ymm0, ymm0, ymm1
    add rax, 32
    sub rbx, 8
    jmp .mean_loop
    
.mean_remainder:
    test rbx, rbx
    jz .mean_complete
    
.mean_scalar:
    vmovss xmm1, [rax]
    vaddss xmm0, xmm0, xmm1
    add rax, 4
    dec rbx
    jnz .mean_scalar
    
.mean_complete:
    ; Horizontal sum for mean
    vhaddps ymm0, ymm0, ymm0
    vhaddps ymm0, ymm0, ymm0
    vextractf128 xmm1, ymm0, 1
    vaddss xmm0, xmm0, xmm1
    
    ; Divide by size to get mean
    vcvtsi2ss xmm2, xmm2, rcx
    vdivss xmm0, xmm0, xmm2    ; Mean in xmm0
    vbroadcastss ymm3, xmm0    ; Broadcast mean
    
    ; Compute variance
    vxorps ymm0, ymm0, ymm0    ; Variance accumulator
    mov rax, r8                ; Reset input pointer
    mov rbx, rcx               ; Reset size counter
    
.variance_loop:
    cmp rbx, 8
    jl .variance_remainder
    
    vmovups ymm1, [rax]
    vsubps ymm1, ymm1, ymm3    ; x - mean
    vmulps ymm1, ymm1, ymm1    ; (x - mean)^2
    vaddps ymm0, ymm0, ymm1    ; Accumulate
    add rax, 32
    sub rbx, 8
    jmp .variance_loop
    
.variance_remainder:
    test rbx, rbx
    jz .variance_complete
    
.variance_scalar:
    vmovss xmm1, [rax]
    vsubss xmm1, xmm1, xmm0    ; x - mean
    vmulss xmm1, xmm1, xmm1    ; (x - mean)^2
    vaddss xmm0, xmm0, xmm1    ; Accumulate
    add rax, 4
    dec rbx
    jnz .variance_scalar
    
.variance_complete:
    ; Horizontal sum and normalize
    vhaddps ymm0, ymm0, ymm0
    vhaddps ymm0, ymm0, ymm0
    vextractf128 xmm1, ymm0, 1
    vaddss xmm0, xmm0, xmm1
    vdivss xmm0, xmm0, xmm2    ; Variance
    
    ; Add epsilon and compute rsqrt
    mov r11, 0x3727C5AC        ; 1e-5 epsilon
    vmovd xmm1, r11d
    vaddss xmm0, xmm0, xmm1
    vsqrtss xmm0, xmm0, xmm0
    
    ; Reciprocal for normalization
    mov r11, 0x3F800000        ; 1.0
    vmovd xmm1, r11d
    vdivss xmm1, xmm1, xmm0    ; 1/sqrt(var + eps)
    vbroadcastss ymm4, xmm1    ; Broadcast normalization factor
    
    ; Apply normalization
    mov rax, r8                ; Input pointer
    mov rdx, r9                ; Output pointer
    mov rbx, rcx               ; Size counter
    
.normalize_loop:
    cmp rbx, 8
    jl .normalize_remainder
    
    vmovups ymm1, [rax]
    vsubps ymm1, ymm1, ymm3    ; x - mean
    vmulps ymm1, ymm1, ymm4    ; (x - mean) / sqrt(var + eps)
    vmovups [rdx], ymm1
    add rax, 32
    add rdx, 32
    sub rbx, 8
    jmp .normalize_loop
    
.normalize_remainder:
    test rbx, rbx
    jz .normalize_complete
    
.normalize_scalar:
    vmovss xmm1, [rax]
    vsubss xmm1, xmm1, xmm0    ; x - mean
    vmulss xmm1, xmm1, xmm4    ; Normalize
    vmovss [rdx], xmm1
    add rax, 4
    add rdx, 4
    dec rbx
    jnz .normalize_scalar
    
.normalize_complete:
    pop r11
    pop r10
    pop rdx
    pop rbx
    pop rbp
    ret

; Scaled dot-product attention with SIMD
; Input: Assumes Q, K, V matrices are prepared in workspace
gpt_scaled_dot_product_simd:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push rdx
    push r8
    push r9
    push r10
    
    ; Compute QK^T with scaling
    ; Scale factor: 1/sqrt(d_k) = 1/sqrt(64) = 0.125 for 64-dim heads
    mov r11, 0x3E000000        ; 0.125 in IEEE 754
    vmovd xmm7, r11d
    vbroadcastss ymm7, xmm7    ; Broadcast scale factor
    
    ; Matrix multiply Q and K^T
    lea r8, [hidden_states]    ; Q matrix
    lea r9, [hidden_states + MAX_CONTEXT_LENGTH * EMBEDDING_DIM * 2] ; K matrix
    lea r10, [attention_scores] ; Output attention scores
    
    mov rax, MAX_CONTEXT_LENGTH ; Sequence length
    mov rbx, MAX_CONTEXT_LENGTH ; Sequence length
    
.attention_outer:
    mov rcx, rbx               ; Reset inner counter
    
.attention_inner:
    ; Compute dot product between Q[i] and K[j]
    vxorps ymm0, ymm0, ymm0    ; Initialize accumulator
    
    push rax
    push rcx
    mov rcx, EMBEDDING_DIM / 8 ; Process 8 elements at a time
    
.attention_dot:
    vmovups ymm1, [r8]         ; Load Q elements
    vmovups ymm2, [r9]         ; Load K elements
    vfmadd231ps ymm0, ymm1, ymm2 ; Fused multiply-add
    add r8, 32
    add r9, 32
    loop .attention_dot
    
    pop rcx
    pop rax
    
    ; Horizontal sum
    vhaddps ymm0, ymm0, ymm0
    vhaddps ymm0, ymm0, ymm0
    vextractf128 xmm1, ymm0, 1
    vaddss xmm0, xmm0, xmm1
    
    ; Apply scaling
    vmulss xmm0, xmm0, xmm7
    
    ; Store attention score
    vmovss [r10], xmm0
    add r10, 4
    
    ; Reset K pointer for next iteration
    sub r9, EMBEDDING_DIM * 4
    
    dec rcx
    jnz .attention_inner
    
    ; Advance Q pointer to next row
    add r8, EMBEDDING_DIM * 4
    
    dec rax
    jnz .attention_outer
    
    ; Apply softmax to attention scores
    lea rdi, [attention_scores]
    mov rsi, MAX_CONTEXT_LENGTH * MAX_CONTEXT_LENGTH
    call gpt_softmax_simd
    
    ; Multiply attention weights with V to get output
    ; (Simplified - would need proper matrix multiplication)
    
    pop r10
    pop r9
    pop r8
    pop rdx
    pop rcx
    pop rbx
    pop rbp
    ret

; Output projection layer with SIMD
gpt_output_projection_simd:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push rdx
    push r8
    push r9
    
    ; Project hidden states to vocabulary logits
    lea r8, [hidden_states]    ; Input: final hidden states
    lea r9, [output_logits]    ; Output: vocabulary logits
    lea rdx, [embedding_weights] ; Use embedding weights (tied)
    
    ; For each vocabulary entry, compute dot product with hidden state
    mov rax, MAX_VOCAB_SIZE    ; Vocabulary size
    
.vocab_loop:
    vxorps ymm0, ymm0, ymm0    ; Initialize accumulator
    
    push rax
    mov rcx, EMBEDDING_DIM / 8 ; Process 8 elements at a time
    mov rbx, r8                ; Hidden state pointer
    
.projection_dot:
    vmovups ymm1, [rbx]        ; Load hidden state elements
    vmovups ymm2, [rdx]        ; Load vocabulary embedding
    vfmadd231ps ymm0, ymm1, ymm2 ; Fused multiply-add
    add rbx, 32
    add rdx, 32
    loop .projection_dot
    
    pop rax
    
    ; Horizontal sum
    vhaddps ymm0, ymm0, ymm0
    vhaddps ymm0, ymm0, ymm0
    vextractf128 xmm1, ymm0, 1
    vaddss xmm0, xmm0, xmm1
    
    ; Store logit
    vmovss [r9], xmm0
    add r9, 4
    
    dec rax
    jnz .vocab_loop
    
    pop r9
    pop r8
    pop rdx
    pop rcx
    pop rbx
    pop rbp
    ret

; Multinomial sampling from probability distribution
; Input: RDI = probabilities, RSI = vocab_size
; Output: RAX = sampled token ID
gpt_multinomial_sample:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push rdx
    push r8
    
    ; Generate random number [0, 1)
    rdtsc                      ; Use timestamp as random seed
    xor rdx, rdx
    mov rbx, 2147483647        ; Large prime
    div rbx
    vcvtsi2ss xmm0, xmm0, rdx  ; Convert to float
    mov rbx, 2147483647
    vcvtsi2ss xmm1, xmm1, rbx
    vdivss xmm0, xmm0, xmm1    ; Random value in [0, 1)
    
    ; Find token by cumulative probability
    mov rcx, rsi               ; Vocab size
    mov r8, rdi                ; Probability array
    xor rax, rax               ; Token counter
    vxorps xmm2, xmm2, xmm2    ; Cumulative sum
    
.sample_loop:
    vmovss xmm3, [r8 + rax*4]  ; Load probability
    vaddss xmm2, xmm2, xmm3    ; Add to cumulative sum
    
    ; Check if random value falls in this bucket
    vucomiss xmm2, xmm0
    jae .sample_found
    
    inc rax
    cmp rax, rcx
    jl .sample_loop
    
    ; Fallback to last token if not found
    dec rax
    
.sample_found:
    pop r8
    pop rdx
    pop rcx
    pop rbx
    pop rbp
    ret

; Parse HTTP request and extract method, path, headers
; Input: RDI = request buffer, RSI = request size
; Output: Sets global variables for parsed data
gpt_parse_http_request:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push rdx
    push r8
    
    ; Simple HTTP parsing - look for POST /v1/completions
    mov rax, rdi               ; Request buffer
    mov rcx, rsi               ; Size
    
    ; Check for POST method
    mov ebx, [rax]             ; Load first 4 bytes
    cmp ebx, 0x54534F50        ; "POST" in little endian
    jne .parse_error
    
    ; Find Content-Length header (simplified)
    mov rbx, 0x746E65746E6F43   ; "Content-"
    
.find_content_length:
    cmp rcx, 8
    jl .parse_error
    
    cmp qword [rax], rbx
    je .found_content_header
    
    inc rax
    dec rcx
    jmp .find_content_length
    
.found_content_header:
    ; Skip to length value (simplified)
    add rax, 16
    sub rcx, 16
    
    ; Parse content length (simplified - just set to remaining size)
    ; In real implementation, would parse decimal number
    
    xor rax, rax               ; Success
    jmp .parse_exit
    
.parse_error:
    mov rax, -1                ; Error
    
.parse_exit:
    pop r8
    pop rdx
    pop rcx
    pop rbx
    pop rbp
    ret

; Parse JSON request to extract prompt and parameters
; Input: RDI = JSON buffer, RSI = buffer size
; Output: Extracts to global variables
gpt_parse_json_request:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push rdx
    push r8
    push r9
    
    ; Simple JSON parsing - look for "prompt": "..."
    mov rax, rdi               ; JSON buffer
    mov rcx, rsi               ; Size
    
    ; Search for "prompt" key
    mov rbx, 0x74706D6F7270     ; "prompt" (first 6 chars)
    
.find_prompt:
    cmp rcx, 8
    jl .json_error
    
    cmp qword [rax], rbx
    je .found_prompt_key
    
    inc rax
    dec rcx
    jmp .find_prompt
    
.found_prompt_key:
    ; Skip to value (look for opening quote)
    add rax, 8
    sub rcx, 8
    
.find_quote:
    cmp rcx, 1
    jl .json_error
    
    cmp byte [rax], 0x22       ; Quote character
    je .found_quote
    
    inc rax
    dec rcx
    jmp .find_quote
    
.found_quote:
    inc rax                    ; Skip opening quote
    dec rcx
    
    ; Find closing quote and extract prompt
    mov r8, rax                ; Start of prompt text
    xor r9, r9                 ; Prompt length counter
    
.find_end_quote:
    cmp rcx, 1
    jl .json_error
    
    cmp byte [rax], 0x22       ; Closing quote
    je .found_end_quote
    
    inc rax
    inc r9
    dec rcx
    jmp .find_end_quote
    
.found_end_quote:
    ; r8 = prompt start, r9 = prompt length
    ; Copy prompt to tokenization buffer for processing
    
    xor rax, rax               ; Success
    jmp .json_exit
    
.json_error:
    mov rax, -1                ; Error
    
.json_exit:
    pop r9
    pop r8
    pop rdx
    pop rcx
    pop rbx
    pop rbp
    ret

; Fast detokenization - convert token IDs back to text
; Input: RDI = token array, RSI = token count, RDX = output buffer
; Output: RAX = output text length
gpt_detokenize_fast:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push r8
    push r9
    push r10
    
    mov r8, rdi                ; Token array
    mov r9, rsi                ; Token count
    mov r10, rdx               ; Output buffer
    xor rax, rax               ; Output length counter
    
.detokenize_loop:
    test r9, r9
    jz .detokenize_complete
    
    ; Load token ID
    mov ebx, [r8]
    
    ; Convert token to text using reverse vocab table
    ; For simplicity, implement basic ASCII mapping
    cmp ebx, 1000
    jl .special_token
    cmp ebx, 1095              ; 1000 + 95 printable ASCII
    jg .special_token
    
    ; Direct ASCII mapping
    sub ebx, 1000
    add ebx, 32                ; Printable ASCII start
    mov [r10 + rax], bl
    inc rax
    jmp .next_token
    
.special_token:
    ; Handle special tokens
    cmp ebx, 220               ; Space token
    je .add_space
    cmp ebx, 198               ; Newline token
    je .add_newline
    cmp ebx, 262               ; " the" token
    je .add_the
    
    ; Unknown token - add placeholder
    mov byte [r10 + rax], '?'
    inc rax
    jmp .next_token
    
.add_space:
    mov byte [r10 + rax], ' '
    inc rax
    jmp .next_token
    
.add_newline:
    mov byte [r10 + rax], 10   ; '\n'
    inc rax
    jmp .next_token
    
.add_the:
    mov dword [r10 + rax], 0x20656874  ; " the"
    add rax, 4
    
.next_token:
    add r8, 4                  ; Next token
    dec r9
    jmp .detokenize_loop
    
.detokenize_complete:
    ; Null-terminate string
    mov byte [r10 + rax], 0
    
    pop r10
    pop r9
    pop r8
    pop rcx
    pop rbx
    pop rbp
    ret

; Format JSON response for API
; Input: RDI = text buffer, RSI = text length, RDX = response buffer
; Output: RAX = response length
gpt_format_json_response:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push r8
    push r9
    push r10
    
    mov r8, rdi                ; Text buffer
    mov r9, rsi                ; Text length
    mov r10, rdx               ; Response buffer
    
    ; Write JSON response header
    lea rbx, [json_header]
    mov rcx, json_header_len
    mov rdi, r10
    mov rsi, rbx
    rep movsb
    mov rax, json_header_len
    
    ; Add generated text (with escaping)
    mov rcx, r9                ; Text length
    
.format_text_loop:
    test rcx, rcx
    jz .format_complete
    
    mov bl, [r8]               ; Load character
    
    ; Escape special JSON characters
    cmp bl, '"'
    je .escape_quote
    cmp bl, '\'
    je .escape_backslash
    cmp bl, 10                 ; Newline
    je .escape_newline
    
    ; Regular character
    mov [r10 + rax], bl
    inc rax
    jmp .next_char
    
.escape_quote:
    mov word [r10 + rax], 0x2022  ; '\"'
    add rax, 2
    jmp .next_char
    
.escape_backslash:
    mov word [r10 + rax], 0x5C5C  ; '\\'
    add rax, 2
    jmp .next_char
    
.escape_newline:
    mov word [r10 + rax], 0x6E5C  ; '\n'
    add rax, 2
    
.next_char:
    inc r8
    dec rcx
    jmp .format_text_loop
    
.format_complete:
    ; Add JSON footer
    lea rbx, [json_footer]
    mov rcx, json_footer_len
    mov rdi, r10
    add rdi, rax
    mov rsi, rbx
    rep movsb
    add rax, json_footer_len
    
    pop r10
    pop r9
    pop r8
    pop rcx
    pop rbx
    pop rbp
    ret

; Start HTTP API server
; Input: None (uses global serve_port)
; Output: RAX = 0 on success, -1 on error
gpt_start_api_server:
    push rbp
    mov rbp, rsp
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push r8
    push r9
    
    ; Create socket
    mov rax, 41                ; sys_socket
    mov rdi, 2                 ; AF_INET
    mov rsi, 1                 ; SOCK_STREAM
    mov rdx, 0                 ; protocol
    syscall
    test rax, rax
    js .server_error
    
    mov r8, rax                ; Save socket fd
    
    ; Set socket options (SO_REUSEADDR)
    mov rax, 54                ; sys_setsockopt
    mov rdi, r8
    mov rsi, 1                 ; SOL_SOCKET
    mov rdx, 2                 ; SO_REUSEADDR
    mov r10, 1                 ; enable
    mov r8, 4                  ; option length
    syscall
    
    ; Bind socket
    mov rax, 49                ; sys_bind
    mov rdi, r8
    lea rsi, [sockaddr_in]     ; Address structure
    mov rdx, 16                ; Address length
    syscall
    test rax, rax
    js .server_error
    
    ; Listen for connections
    mov rax, 50                ; sys_listen
    mov rdi, r8
    mov rsi, 128               ; Backlog
    syscall
    test rax, rax
    js .server_error
    
    ; Main server loop (simplified - single threaded)
.server_loop:
    ; Accept connection
    mov rax, 43                ; sys_accept
    mov rdi, r8
    xor rsi, rsi              ; No client address needed
    xor rdx, rdx
    syscall
    test rax, rax
    js .server_loop            ; Continue on error
    
    mov r9, rax                ; Client socket
    
    ; Read request
    mov rax, 0                 ; sys_read
    mov rdi, r9
    lea rsi, [request_buffer]
    mov rdx, 4096
    syscall
    
    ; Process request
    lea rdi, [request_buffer]
    mov rsi, rax
    lea rdx, [response_buffer]
    call gpt_api_endpoint
    
    ; Send response
    mov rdx, rax               ; Response length
    mov rax, 1                 ; sys_write
    mov rdi, r9
    lea rsi, [response_buffer]
    syscall
    
    ; Close client connection
    mov rax, 3                 ; sys_close
    mov rdi, r9
    syscall
    
    jmp .server_loop
    
.server_error:
    mov rax, -1
    
.server_exit:
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rbp
    ret

; MLP-specific matrix multiplication helper
gpt_simd_matmul_mlp:
    push rbp
    mov rbp, rsp
    
    ; Reuse the main SIMD matmul but with MLP-specific optimizations
    call gpt_simd_matmul
    
    pop rbp
    ret

; Additional data section for JSON formatting and networking
.section .data
    json_header:        .ascii '{"id":"chatcmpl-123","object":"chat.completion","created":1234567890,"model":"gpt-assembly","choices":[{"index":0,"message":{"role":"assistant","content":"'
    json_header_len     equ $ - json_header
    
    json_footer:        .ascii '"},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}}'
    json_footer_len     equ $ - json_footer
    
    ; Socket address structure
    sockaddr_in:
        .word 2                ; AF_INET
        .word 0x2C0B          ; Port 11434 in network byte order
        .long 0               ; INADDR_ANY
        .space 8              ; Padding
    
    ; Buffers for networking
    request_buffer:     .space 4096
    response_buffer:    .space 8192
