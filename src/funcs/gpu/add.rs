use super::*;

pub fn gpu_run(out_id: TensorId, inps: &[Vec<usize>]) -> GpuFunction {
    let inp0_size = inps[0].iter().fold(1, |a, b| a * b);
    let inp1_size = inps[1].iter().fold(1, |a, b| a * b);
    let works = std::cmp::max(inp0_size, inp1_size);
    let source_code = format!(
        "__kernel void calc_{out_id}(
                        __global float* out,
                        __global float* a,
                        __global float* b) {{
        uint id = get_global_id(0);
        uint id_a = id % {inp0_size};
        uint id_b = id % {inp1_size};
        if(id < {works}) {{
            out[id] = a[id_a] + b[id_b];
        }}
    }}"
    );

    let local_work_size = 32;
    let global_work_size =
        works + ((local_work_size - (works % local_work_size)) % local_work_size);

    GpuFunction {
        source_code,
        kernel_name: format!("calc_{}", out_id),
        local_work_size,
        global_work_size,
    }
}

pub fn gpu_grad(out_id: TensorId, inps: &[Vec<usize>]) -> GpuFunctionGroup {
    let inp0_size = inps[0].iter().fold(1, |a, b| a * b);
    let inp1_size = inps[1].iter().fold(1, |a, b| a * b);
    assert!(inp1_size <= inp0_size);
    let repeats = inp0_size / inp1_size;
    let works = std::cmp::max(inp0_size, inp1_size);
    let source_code = format!(
        "__kernel void grad_{out_id}_1(
                        __global float* out,
                        __global float* out_grad,
                        __global float* grad_buff,
                        __global float* a,
                        __global float* a_grad,
                        __global float* b,
                        __global float* b_grad) {{
        uint id = get_global_id(0);
        if(id < {works}) {{
            a_grad[id] += out_grad[id];
        }}
    }}"
    );

    let source_code_2 = format!(
        "__kernel void grad_{out_id}_2(
                        __global float* out,
                        __global float* out_grad,
                        __global float* grad_buff,
                        __global float* a,
                        __global float* a_grad,
                        __global float* b,
                        __global float* b_grad) {{
        uint id = get_global_id(0);
        if(id < {inp1_size}) {{
            for(uint i = 0; i < {repeats}; i++) {{
                b_grad[id] += out_grad[i * {inp1_size} + id];
            }}
        }}
    }}"
    );

    GpuFunctionGroup {
        shared_buffers: vec![works],
        funcs: vec![
            GpuFunction {
                source_code,
                kernel_name: format!("grad_{}_1", out_id),
                local_work_size: 32,
                global_work_size: works + ((32 - (works % 32)) % 32),
            },
            GpuFunction {
                source_code: source_code_2,
                kernel_name: format!("grad_{}_2", out_id),
                local_work_size: 32,
                global_work_size: inp1_size + ((32 - (inp1_size % 32)) % 32),
            },
        ],
    }
}