use ash::vk;

pub(crate) fn create_swapchain_with_status(
    initial: bool,
    mut create: impl FnMut(&mut vk::SwapchainKHR) -> vk::Result,
) -> Result<vk::SwapchainKHR, vk::Result> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        let mut raw = vk::SwapchainKHR::null();
        let status = create(&mut raw);
        if status == vk::Result::SUCCESS {
            return if raw == vk::SwapchainKHR::null() {
                Err(vk::Result::ERROR_INITIALIZATION_FAILED)
            } else {
                Ok(raw)
            };
        }

        // NVIDIA/Wayland can return this undocumented status without a handle
        // during popup mapping. An old swapchain may already have been retired,
        // so retry only initial creation and never use an unsuccessful output.
        if initial
            && status == vk::Result::SUBOPTIMAL_KHR
            && raw == vk::SwapchainKHR::null()
            && attempts < 3
        {
            log::warn!(
                "vkCreateSwapchainKHR returned SUBOPTIMAL_KHR without a handle; retrying initial creation (attempt {attempts}/3)"
            );
            continue;
        }
        return Err(status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ash::vk::Handle;

    #[test]
    fn transient_null_suboptimal_retries_initial_creation() {
        let expected = vk::SwapchainKHR::from_raw(17);
        let mut attempts = 0;
        let result = create_swapchain_with_status(true, |raw| {
            attempts += 1;
            if attempts == 1 {
                vk::Result::SUBOPTIMAL_KHR
            } else {
                *raw = expected;
                vk::Result::SUCCESS
            }
        });
        assert_eq!(result, Ok(expected));
        assert_eq!(attempts, 2);
    }

    #[test]
    fn success_preserves_returned_swapchain() {
        let expected = vk::SwapchainKHR::from_raw(18);
        assert_eq!(
            create_swapchain_with_status(true, |raw| {
                *raw = expected;
                vk::Result::SUCCESS
            }),
            Ok(expected)
        );
    }

    #[test]
    fn device_loss_and_allocation_errors_are_not_retried() {
        for error in [
            vk::Result::ERROR_DEVICE_LOST,
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY,
            vk::Result::ERROR_OUT_OF_HOST_MEMORY,
        ] {
            let mut attempts = 0;
            assert_eq!(
                create_swapchain_with_status(true, |_| {
                    attempts += 1;
                    error
                }),
                Err(error)
            );
            assert_eq!(attempts, 1);
        }
    }

    #[test]
    fn null_success_is_not_used() {
        assert_eq!(
            create_swapchain_with_status(true, |_| vk::Result::SUCCESS),
            Err(vk::Result::ERROR_INITIALIZATION_FAILED)
        );
    }

    #[test]
    fn existing_swapchain_is_not_retried() {
        let mut attempts = 0;
        assert_eq!(
            create_swapchain_with_status(false, |_| {
                attempts += 1;
                vk::Result::SUBOPTIMAL_KHR
            }),
            Err(vk::Result::SUBOPTIMAL_KHR)
        );
        assert_eq!(attempts, 1);
    }

    #[test]
    fn nonnull_suboptimal_is_not_retried_or_used() {
        let mut attempts = 0;
        assert_eq!(
            create_swapchain_with_status(true, |raw| {
                attempts += 1;
                *raw = vk::SwapchainKHR::from_raw(19);
                vk::Result::SUBOPTIMAL_KHR
            }),
            Err(vk::Result::SUBOPTIMAL_KHR)
        );
        assert_eq!(attempts, 1);
    }

    #[test]
    fn repeated_suboptimal_has_a_fixed_attempt_limit() {
        let mut attempts = 0;
        assert_eq!(
            create_swapchain_with_status(true, |_| {
                attempts += 1;
                vk::Result::SUBOPTIMAL_KHR
            }),
            Err(vk::Result::SUBOPTIMAL_KHR)
        );
        assert_eq!(attempts, 3);
    }

    #[test]
    fn third_attempt_can_succeed() {
        let mut attempts = 0;
        let expected = vk::SwapchainKHR::from_raw(20);
        assert_eq!(
            create_swapchain_with_status(true, |raw| {
                attempts += 1;
                if attempts < 3 {
                    vk::Result::SUBOPTIMAL_KHR
                } else {
                    *raw = expected;
                    vk::Result::SUCCESS
                }
            }),
            Ok(expected)
        );
        assert_eq!(attempts, 3);
    }
}
